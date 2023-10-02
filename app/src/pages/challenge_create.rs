use crate::functions::challenges::create::CreateChallenge;
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn ChallengeCreate() -> impl IntoView {
    let create_game_action = create_server_action::<CreateChallenge>();
    view! {
        <ActionForm action=create_game_action>
            <input type="checkbox" name="public" value="Public"/>
            <input type="checkbox" name="rated" value="Rated"/>
            <input type="checkbox" name="tournament_queen_rule" value="Tournament rules" />
            <select name="color_choice">
                <option value="Random">"Random"</option>
                <option value="White">"White"</option>
                <option value="Black">"Black"</option>
            </select>
            <select name="color_choice">
                <option value="MLP">"PLM"</option>
                <option value="Base">"Base"</option>
            </select>
            <input
                type="submit"
                class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline"
                value="Create new challenge"
            />
        </ActionForm>
    }
}
