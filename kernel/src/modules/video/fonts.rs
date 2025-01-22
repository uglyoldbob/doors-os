//! Contains kernel fonts

use core::marker::PhantomData;

/// A fixed width font
pub struct FixedWidthFont<P> {
    _phantom: PhantomData<P>,
}

impl<P> super::FontTrait<P> for FixedWidthFont<P>
where
    P: Sync + Send,
{
    fn lookup_symbol(&self, c: char) -> Option<&super::FontData> {
        todo!()
    }

    fn height(&self) -> u16 {
        todo!()
    }

    fn symbols(&self) -> alloc::collections::btree_map::Iter<char,super::FontData> {
        todo!()
    }
}

/// A fixed width font
pub struct VariableWidthFont<P> {
    fdata: &'static alloc::collections::BTreeMap<char, super::FontData>,
    _phantom: PhantomData<P>,
}

impl<P> VariableWidthFont<P> {
    /// Create a new variable width font
    pub fn new(fdata: &'static alloc::collections::BTreeMap<char, super::FontData>) -> Self {
        Self {
            fdata,
            _phantom: PhantomData,
        }
    }
}

impl<P> super::FontTrait<P> for VariableWidthFont<P>
where
    P: Sync + Send,
{
    fn lookup_symbol(&self, c: char) -> Option<&super::FontData> {
        if self.fdata.contains_key(&c) {
            let d = self.fdata.get(&c).unwrap();
            Some(d)
        } else {
            None
        }
    }

    fn height(&self) -> u16 {
        todo!()
    }

    fn symbols(&self) -> alloc::collections::btree_map::Iter<char, super::FontData> {
        self.fdata.iter()
    }
}
