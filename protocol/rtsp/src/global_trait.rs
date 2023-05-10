pub trait TMsgConverter {
    fn unmarshal(request_data: &str) -> Option<Self>
    where
        Self: Sized;
    fn marshal(&self) -> String;
}
