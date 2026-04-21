// taxus-common/src/hooks/custom_hooks.rs

use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use yew::prelude::*;

#[hook]
pub fn use_click_outside(on_outside_click: Callback<()>) -> NodeRef {
    let node_ref = use_node_ref();

    use_effect_with((), {
        let node_ref = node_ref.clone();
        let on_outside_click = on_outside_click.clone();
        move |_| {
            let document = web_sys::window()
                .expect("no window")
                .document()
                .expect("no document");

            let listener = Closure::<dyn Fn(web_sys::Event)>::new({
                let node_ref = node_ref.clone();
                let on_outside_click = on_outside_click.clone();
                move |event: web_sys::Event| {
                    if let Some(target) = event.target()
                        && let Some(node) = node_ref.cast::<web_sys::Node>()
                        && !node.contains(Some(&target.unchecked_into::<web_sys::Node>()))
                    {
                        on_outside_click.emit(());
                    }
                }
            });

            document
                .add_event_listener_with_callback("click", listener.as_ref().unchecked_ref())
                .expect("failed to add click listener");

            move || {
                document
                    .remove_event_listener_with_callback("click", listener.as_ref().unchecked_ref())
                    .ok();
                drop(listener);
            }
        }
    });

    node_ref
}
