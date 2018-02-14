pub trait Decode {
    type Item;

    fn decode(&self, buffer: &mut Vec<u8>) -> Option<Self::Item>;
}

pub trait Encode {
    type Item;

    fn encode(&self, item: Self::Item, buffer: &mut Vec<u8>);
}
