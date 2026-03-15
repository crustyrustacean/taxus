// common/src/components/page.rs

// dependencies
use crate::Layout;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct PageProps {
    pub title: String,
    pub content: AttrValue,
}

#[component]
pub fn Page(props: &PageProps) -> Html {
    let content = Html::from_html_unchecked(props.content.clone());

    html! {
        <Layout>
            <section>
                <h2>{ &props.title }</h2>
                    { content }
            </section>
        </Layout>
    }
}
