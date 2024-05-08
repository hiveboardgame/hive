use crate::components::molecules::banner::Banner;
use crate::components::organisms::online_users::OnlineUsers;
use crate::components::organisms::tv::Tv;
use crate::providers::auth_context::AuthContext;
use crate::{
    components::{molecules::modal::Modal, organisms::challenges::Challenges},
    pages::challenge_create::ChallengeCreate,
};
use leptos::{html::Dialog, *};
use leptos_router::use_navigate;

#[component]
pub fn Home() -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let open = create_rw_signal(false);
    let dialog_el = create_node_ref::<Dialog>();
    let close_modal = Callback::new(move |()| {
        dialog_el
            .get_untracked()
            .expect("dialog to have been created")
            .close();
    });
    let logged_in = move || matches!((auth_context.user)(), Some(Ok(Some(_))));
    let logo = move || {
        view! { <img width="100%" height="100%" src="/assets/favicon-32x32.png" alt="ladybug"/> }
    };

    view! {
        <div class="pt-20 flex flex-col justify-start md:justify-center items-center w-full overflow-x-clip">
            <Banner title="hivegame.com" extend_tw_classes="w-10/12" logo=logo()/>
            <button
                class="m-5 grow md:grow-0 whitespace-nowrap bg-ant-blue hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded"
                on:click=move |_| {
                    if logged_in() {
                        open.update(move |b| *b = true)
                    } else {
                        let navigate = use_navigate();
                        navigate("/login", Default::default());
                    }
                }
            >
                Play
            </button>
            <div class="flex flex-col md:flex-row justify-center items-center">
                <Modal open=open dialog_el=dialog_el>
                    <ChallengeCreate close=close_modal/>
                </Modal>
                <div class="flex flex-col md:flex-row w-full items-center md:items-start">
                    <div class="flex flex-col">
                        <Challenges/>
                        <Tv/>
                    </div>
                    <OnlineUsers/>
                </div>
            </div>
        </div>
    }
}
