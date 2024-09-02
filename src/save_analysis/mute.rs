use super::defs::CrateDisambiguator;
use crate::StrErr;
use std::path::{Path, PathBuf};
use std::iter::FromIterator;
use intervaltree::IntervalTree;
use chashmap::CHashMap;


// TODO the number of all macro spans that were ever handled
// in servo is about 19k. This means our list is bounded by
// that size. Tbh at these ranges it might be more efficient
// to forego building the interval tree. It might also help
// with code complexity.
struct MuteSpans {
	inner :IntervalTree<(usize, usize), MuteSpan>,
}

type MuteSpan = ((usize, usize), (usize, usize));

impl MuteSpans {
	fn search(&self, needle :&super::defs::Span) -> impl Iterator<Item=MuteSpan> + '_ {
		let needle_start = (needle.line_start as usize, needle.column_start as usize);
		let needle_end = (needle.line_end as usize, needle.column_end as usize);
		self.inner.query(needle_start..needle_end).map(|el|el.value)
	}
}
impl FromIterator<MuteSpan> for MuteSpans {
	fn from_iter<T: IntoIterator<Item = MuteSpan>>(iter :T) -> Self {
		Self {
			inner : <IntervalTree<_,_> as FromIterator<_>>::from_iter(iter.into_iter().map(|v| {
				intervaltree::Element {
					range : (v.0..v.1),
					value : v,
				}
			}))
		}
	}
}

fn in_mute_spans(mute_spans :&MuteSpans, needle_span :&super::defs::Span) -> bool {
	for (start, end) in mute_spans.search(needle_span) {
		let needle_start = (needle_span.line_start as usize, needle_span.column_start as usize);
		let needle_end = (needle_span.line_end as usize, needle_span.column_end as usize);
		if start <= needle_start
				&& end >= needle_end {
			info!("{}:{}:{}: unused ignored because of macro: {:?} till {:?}",
				needle_span.file_name,
				needle_span.line_start, needle_span.column_start,
				start, end);
			return true;
		}
	}
	false
}

pub struct MuteSpansCache {
	prefix :PathBuf,
	cache :CHashMap<(CrateDisambiguator, String), MuteSpans>,
}

impl MuteSpansCache {
	pub fn new<'a>(prefix :impl Into<&'a Path>) -> Self {
		Self {
			prefix : prefix.into().to_owned(),
			cache : CHashMap::new(),
		}
	}
	pub fn is_in_macro(&self, crate_id :CrateDisambiguator, needle_span :&super::defs::Span) -> Result<bool, StrErr> {
		if let Some(mute_spans) = self.cache.get(&(crate_id, needle_span.file_name.clone())) {
			return Ok(in_mute_spans(&mute_spans, needle_span));
		}
		let mut path = self.prefix.clone();
		path.push(&needle_span.file_name);
		let file = std::fs::read_to_string(path)?;
		let mute_spans = mute_spans_for_file(&file)?;

		let ret = in_mute_spans(&mute_spans, needle_span);
		self.cache.insert((crate_id, needle_span.file_name.clone()), mute_spans);
		Ok(ret)
	}
}

fn mute_spans_for_file<'a>(file :&str) -> Result<MuteSpans, StrErr> {
	use syn::parse::Parser;
	use syn::parse::ParseStream;
	use syn::{Attribute, Item, Macro, ItemFn, Ident};
	use syn::spanned::Spanned;
	use syn::visit::{visit_item, self};
	use proc_macro2::{LineColumn, Span};
	struct Visitor<'a> {
		mute_spans :&'a mut Vec<MuteSpan>,
	}
	fn lc(v :LineColumn) -> (usize, usize) {
		// Columns are 0-based for some reason...
		// https://github.com/rust-lang/rust/issues/54725
		(v.line, v.column + 1)
	}
	fn span_min_max<Sp :Spanned>(first :MuteSpan,
			it :impl Iterator<Item=Sp>) -> MuteSpan {
		it.fold(first, |(m_start, m_end), ntt| {
				let sp = ntt.span();
				(m_start.min(lc(sp.start())), m_end.max(lc(sp.end())))
			})
	}
	impl<'ast, 'a> syn::visit::Visit<'ast> for Visitor<'a> {
		fn visit_macro(&mut self, m :&'ast Macro) {
			let sp = m.span();

			// We need to find the maximum span encompassing the entire
			// macro. m.span() only points to the macro's name.
			// Thus, iterate over the entire macro's invocation.
			let start = lc(sp.start());
			let end = lc(sp.end());
			let (start, end) = span_min_max((start, end), m.tokens.clone().into_iter());

			self.mute_spans.push((start, end));
		}
		fn visit_item_fn(&mut self, i :&'ast ItemFn) {
			let fn_name = &i.sig.ident;
			let has_proc_macr_attr = i.attrs.iter()
				.any(|a| {
					let proc_macro_id = Ident::new("proc_macro", Span::call_site());
					let proc_macro_der_id = Ident::new("proc_macro_derive", Span::call_site());
					let proc_macro_attr_id = Ident::new("proc_macro_attribute", Span::call_site());
					let p = &a.path;
					let is_proc_macro = p.is_ident(&proc_macro_id) ||
						p.is_ident(&proc_macro_der_id) ||
						p.is_ident(&proc_macro_attr_id);
					if is_proc_macro {
						error!("Found proc macro {}", fn_name);
					}
					is_proc_macro
				});
			if has_proc_macr_attr {
				let sp = i.sig.output.span();
				let start = lc(sp.start());
				let end = lc(sp.end());
				let (start, end) = span_min_max((start, end),
					i.attrs.clone().into_iter());
				self.mute_spans.push((start, end));
			}
			visit::visit_item_fn(self, i);
		}
		fn visit_attribute(&mut self, a :&'ast Attribute) {
			let sp = a.span();

			// We need to find the maximum span encompassing the entire
			// macro. m.span() only points to the macro's name.
			// Thus, iterate over the entire macro's invocation.
			let start = lc(sp.start());
			let end = lc(sp.end());
			let (start, end) = span_min_max((start, end), a.tokens.clone().into_iter());

			self.mute_spans.push((start, end));
		}
	}
	let (_attrs, items) = (|stream :ParseStream| {
		let attrs = stream.call(Attribute::parse_inner)?;
		let mut items = Vec::new();
		while !stream.is_empty() {
			let item :Item = stream.parse()?;
			items.push(item);
		}
		Ok((attrs, items))
	}).parse_str(file)?;


	let mut mute_spans_vec = Vec::new();
	for item in items.iter() {
		let mut visitor = Visitor {
			mute_spans : &mut mute_spans_vec,
		};
		visit_item(&mut visitor, &item);
	}
	let mute_spans = MuteSpans::from_iter(mute_spans_vec);
	return Ok(mute_spans);
}
