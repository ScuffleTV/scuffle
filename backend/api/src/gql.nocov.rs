#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod api;
mod config;
mod database;
mod dataloader;
mod global;
mod pb;

use api::v1::gql::schema;
use async_graphql::SDLExportOptions;

fn main() {
    let schema = schema();

    println!(
        "{}",
        schema.sdl_with_options(
            SDLExportOptions::default()
                .federation()
                .include_specified_by()
                .sorted_arguments()
                .sorted_enum_items()
                .sorted_fields()
        )
    );
}
