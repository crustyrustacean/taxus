// common/src/components/layout.rs

// dependencies
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct LayoutProps {
    pub children: Html,
}

#[component]
pub fn Layout(props: &LayoutProps) -> Html {
    html! {
        <>
            <header>
                <h1>{ "Site Name" }</h1>
            </header>

            <nav>
                <a href="/">{ "Home" }</a>
                <a href="/about">{ "About" }</a>
            </nav>

            <main>
                { props.children.clone() }
            </main>

            <footer>
                <p>{ "Built with Yew SSG" }</p>
            </footer>
        </>
    }
}
