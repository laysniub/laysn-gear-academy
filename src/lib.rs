#![no_std]

use gstd::{debug, exec, msg};
use pebbles_game_io::*;

static mut GAME_CONTEXT: Option<GameState> = None;

#[cfg(test)]
fn get_random_u32() -> u32 {
    2
}

#[cfg(not(test))]
fn get_random_u32() -> u32 {
    let seed = msg::id();
    let (hash, _) = exec::random(seed.into()).expect("get_random_u32(): random call failed");
    u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]])
}

fn player_ch() -> Player {
    if get_random_u32() % 2 == 0 {
        Player::Program
    } else {
        Player::User
    }
}

fn program(state: &GameState) -> u32 {
    let max = state.max_pebbles_per_turn;
    match state.difficulty {
        DifficultyLevel::Easy => get_random_u32() % max + 1,
        DifficultyLevel::Hard => {
            let optimal = state.pebbles_remaining % (max + 1);
            if optimal == 0 {
                1
            } else {
                optimal
            }
        }
    }
}

fn move_s(state: &mut GameState, amount: u32, player: Player) -> PebblesEvent {
    state.pebbles_remaining = state.pebbles_remaining.saturating_sub(amount);
    if state.pebbles_remaining == 0 {
        state.winner = Some(player.clone());
        PebblesEvent::Won(player)
    } else {
        PebblesEvent::CounterTurn(amount)
    }
}

fn program_turn(state: &mut GameState) -> PebblesEvent {
    let amount = program(state).min(state.pebbles_remaining);
    debug!("AI removes {} pebbles", amount);
    move_s(state, amount, Player::Program)
}

fn initialize_game(config: PebblesInit) -> GameState {
    if config.pebbles_count == 0 || config.max_pebbles_per_turn == 0 {
        panic!("Invalid game configuration");
    }
    let first_player = player_ch();
    let mut state = GameState {
        pebbles_count: config.pebbles_count,
        max_pebbles_per_turn: config.max_pebbles_per_turn,
        pebbles_remaining: config.pebbles_count,
        difficulty: config.difficulty,
        first_player: first_player.clone(),
        winner: None,
    };
    if let Player::Program = first_player {
        let _ = program_turn(&mut state);
    }
    state
}

fn human_turn(state: &mut GameState, amount: u32) -> PebblesEvent {
    if amount == 0 || amount > state.max_pebbles_per_turn {
        panic!("Invalid move");
    }
    debug!("Human removes {} pebbles", amount);
    let event = move_s(state, amount, Player::User);
    if matches!(event, PebblesEvent::CounterTurn(_)) {
        program_turn(state)
    } else {
        event
    }
}

#[no_mangle]
extern "C" fn init() {
    let config: PebblesInit = msg::load().expect("Failed to decode init config");
    debug!("Game initialization: {:?}", config);
    let state = initialize_game(config);
    unsafe {
        GAME_CONTEXT = Some(state);
    }
}

#[allow(static_mut_refs)]
#[no_mangle]
extern "C" fn handle() {
    let action: PebblesAction = msg::load().expect("Failed to decode action");
    debug!("Received action: {:?}", action);
    let state = unsafe { GAME_CONTEXT.as_mut().expect("Game not initialized") };

    let event = match action {
        PebblesAction::Turn(amount) => {
            if state.winner.is_some() {
                panic!("Game is already finished. Please restart the game.");
            }
            human_turn(state, amount)
        }
        PebblesAction::GiveUp => {
            if state.winner.is_some() {
                panic!("Game is already finished. Please restart the game.");
            }
            state.winner = Some(Player::Program);
            PebblesEvent::Won(Player::Program)
        }
        PebblesAction::Restart {
            difficulty,
            pebbles_count,
            max_pebbles_per_turn,
        } => {
            *state = initialize_game(PebblesInit {
                difficulty,
                pebbles_count,
                max_pebbles_per_turn,
            });
            return;
        }
    };
    msg::reply(event, 0).expect("Failed to send game event");
}

#[allow(static_mut_refs)]
#[no_mangle]
extern "C" fn state() {
    let state = unsafe { GAME_CONTEXT.as_ref().expect("Game not initialized") };
    msg::reply(state, 0).expect("Failed to send game state");
}
