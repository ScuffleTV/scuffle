use scuffle_foundations::settings::{auto_settings, Settings};

#[auto_settings]
pub struct BaseSettings<S: Settings + Default> {
    #[serde(flatten)]
    /// The internal settings.
    external: S,
}

#[auto_settings]
pub struct ExtraSettings {
    /// An extra setting.
    pub extra: bool,
    /// Another extra setting.
    pub another: bool,
}

fn main() {
    println!("{}", BaseSettings::<ExtraSettings>::default().to_yaml_string().unwrap());
}
