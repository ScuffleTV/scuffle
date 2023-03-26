#[macro_export]
macro_rules! vec_of_strings {
    ($($x:expr),* $(,)?) => (vec![$($x.into()),*] as Vec<String>);
}
