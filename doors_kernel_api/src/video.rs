/// This trait is used for text only video output hardware
pub trait TextDisplay: Sync + Send {
    /// Write a single character to the video hardware
    fn print_char(&mut self, d: char);

    /// Write an array of characters to the video hardware
    fn print_str(&mut self, d: &str) {
        for c in d.chars() {
            self.print_char(c);
        }
    }

    /// Repeatedly prints a given character a certain number of times
    fn print_repeat_letter(&mut self, d: char, n: u8) {
        for _ in 0..=n {
            self.print_char(d);
        }
    }
}
