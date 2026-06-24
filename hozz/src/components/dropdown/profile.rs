use std::sync::Arc;

use dioxus::{logger::tracing, prelude::*};
use shared::{app::orchestrator::Orchestrator, infra::ProfileManager};

use crate::{
    components::{item::ProfileItem, pet::ZeroEmpty},
    pages::app::AppState,
};

#[component]
pub(crate) fn ProfileDropdownItems() -> Element {
    let app = use_context::<AppState>();
    let profiles = (app.profiles)();
    let mut syncing_profile_id = use_signal(|| None::<String>);

    use_future(move || async move {
        if let Err(e) = consume_context::<Arc<Orchestrator>>()
            .sync_profiles()
            .await
        {
            tracing::warn!(error = %e, "Failed sync profiles");
        }
    });

    rsx! {
        if profiles.is_empty() {
            ZeroEmpty {
                title: "ЧИСТЫЙ ЭФИР И ЧИСТАЯ СОВЕСТЬ",
                description: "У вас пока нет подписок. Добавьте подписку и станьте частью сообщества."
            }
        } else {{
            profiles
                .iter()
                .map(|profile| {
                    rsx! { ProfileItem {
                        key: "{profile.id}",
                        profile: profile.clone(),
                        is_updating: syncing_profile_id() == Some(profile.id.clone()),
                        onupdate: move |id: String| {
                            spawn(async move {
                                syncing_profile_id.set(Some(id.clone()));
                                consume_context::<Arc<Orchestrator>>().update_profile(&id).await;
                                syncing_profile_id.set(None);
                            });
                        },
                        onhealth: move |id: String| {
                            spawn(async move {
                                syncing_profile_id.set(Some(id.clone()));
                                consume_context::<Arc<Orchestrator>>().health_profile(&id).await;
                                syncing_profile_id.set(None);
                            });
                        },
                        ondelete: move |id: String| {
                            spawn(async move {
                                let _ = consume_context::<Arc<Orchestrator>>()
                                    .delete_profile(&id).await;
                            });
                        },
                        ontoggle: move |id: String| {
                            spawn(async move {
                                syncing_profile_id.set(Some(id.clone()));
                                let _ = consume_context::<Arc<Orchestrator>>()
                                    .toggle_profile(&id)
                                    .await;
                                syncing_profile_id.set(None);
                            });
                        }
                    }}
                })
            }}
    }
}
