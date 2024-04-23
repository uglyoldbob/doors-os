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
    fn lookup_symbol(&self, c: char) -> super::OpaqueFrameBuffer<P> {
        todo!()
    }

    fn height(&self) -> u16 {
        todo!()
    }
}
