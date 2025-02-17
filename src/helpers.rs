pub trait Capitalize {
    fn capitalize(&self) -> String;
}

impl Capitalize for String {
    fn capitalize(&self) -> String {
        let mut chars = self.chars();
        match chars.next() {
            Some(first) => {
                first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
            }
            None => String::new(),
        }
    }
}
