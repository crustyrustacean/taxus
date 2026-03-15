// Common library code

// module declarations
pub mod components;

// re-exports
pub use components::*;

// dependencies
use yew::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Route {
    Home,
    About,
}

pub fn switch(route: &Route, title: String, content: AttrValue) -> Html {
    match route {
        Route::Home | Route::About => html! { <Page {title} {content} /> },
    }
}
