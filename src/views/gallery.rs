use dioxus::dioxus_core::Task;
use dioxus::document::eval;
use crate::components::{Echo, Hero};
use dioxus::prelude::*;

pub fn Gallery() -> Element {
    rsx! {
        div { id: "title",
            h1 { "gallery" }
        }
        // div { id: "dogview",
        //     img { src: "https://images.dog.ceo/breeds/pitbull/dog-3981540_1280.jpg" }
        // }
        // div { id: "buttons",
        //     button { id: "skip", "skip" }
        //     button { id: "save", "save!" }
        // }
    }
}