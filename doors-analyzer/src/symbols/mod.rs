use object::{Object, ObjectSymbol};

pub enum Symbol {
    Address(usize),
}

impl Symbol {
    pub fn address(&self) -> usize {
        match self {
            Self::Address(a) => *a,
        }
    }
}

#[enum_dispatch::enum_dispatch]
pub trait SymbolsTrait {
    fn find_symbol<F: Fn(&str) -> String>(&self, symbol: &str, f: F) -> Option<Symbol>;
}

impl<'a> SymbolsTrait for object::File<'a> {
    fn find_symbol<F: Fn(&str) -> String>(&self, symbol: &str, f: F) -> Option<Symbol> {
        for s in self.symbols() {
            if let Ok(sn) = s.name() {
                if symbol == f(sn) {
                    return Some(Symbol::Address(s.address() as usize));
                }
            }
        }
        None
    }
}

#[enum_dispatch::enum_dispatch(SymbolsTrait)]
pub enum Symbols<'a> {
    ObjectSymbols(object::File<'a>),
}

impl<'a> Symbols<'a> {
    pub fn load(data: &'a [u8]) -> Result<Self, String> {
        object::File::parse(data)
            .map(|a| a.into())
            .map_err(|e| e.to_string())
    }
}
