use gtest::{Log, Program, System};
use pebbles_game_io::*;

const PLAYER: u64 = 100;

fn init_game(
    sys: &System,
    difficulty: DifficultyLevel,
    pebbles_count: u32,
    max_pebbles_per_turn: u32,
) -> Program<'_> {
    sys.init_logger();
    let program = Program::current(sys);
    sys.mint_to(PLAYER, 100_000_000_000_000);

    let pebbles_init = PebblesInit {
        difficulty,
        pebbles_count,
        max_pebbles_per_turn,
    };
    let _res = program.send(PLAYER, pebbles_init);
    sys.run_next_block();

    program
}

#[test]
fn game_flow() {
    let sys = System::new();
    let game = init_game(&sys, DifficultyLevel::Easy, 15, 2);

    // Check initial state
    let state: GameState = game.read_state(b"").unwrap();
    assert_eq!(state.pebbles_count, 15);
    assert_eq!(state.max_pebbles_per_turn, 2);
    assert!(state.pebbles_remaining <= 15 && state.pebbles_remaining >= 13);
    assert!(matches!(state.difficulty, DifficultyLevel::Easy));
    assert!(state.winner.is_none());

    // Player's turn
    let _res = game.send(PLAYER, PebblesAction::Turn(1));
    sys.run_next_block();

    let _expected_counter = Log::builder().payload(PebblesEvent::CounterTurn(1));

    // Check if game is over
    let state: GameState = game.read_state(b"").unwrap();
    if state.winner.is_some() {
        assert!(matches!(state.winner, Some(Player::Program)));
    } else {
        // Continue game until finished
        loop {
            let state: GameState = game.read_state(b"").unwrap();
            if state.winner.is_some() {
                break;
            }
            let pebbles_to_remove =
                std::cmp::min(state.pebbles_remaining, state.max_pebbles_per_turn);
            let _res = game.send(PLAYER, PebblesAction::Turn(pebbles_to_remove));
            sys.run_next_block();
        }
    }

    // Check final state
    let final_state: GameState = game.read_state(b"").unwrap();
    assert!(final_state.winner.is_some());
    assert_eq!(final_state.pebbles_remaining, 0);
}

#[test]
fn difficulty_levels() {
    let sys = System::new();

    // Easy mode
    let easy_game = init_game(&sys, DifficultyLevel::Easy, 15, 2);
    let easy_state: GameState = easy_game.read_state(b"").unwrap();
    assert!(matches!(easy_state.difficulty, DifficultyLevel::Easy));

    // Hard mode
    let hard_game = init_game(&sys, DifficultyLevel::Hard, 15, 2);
    let hard_state: GameState = hard_game.read_state(b"").unwrap();
    assert!(matches!(hard_state.difficulty, DifficultyLevel::Hard));
}

#[test]
fn invalid_inputs() {
    let sys = System::new();
    let program = Program::current(&sys);
    sys.mint_to(PLAYER, 100_000_000_000_000);

    // Invalid initialization parameters
    let invalid_init = PebblesInit {
        difficulty: DifficultyLevel::Easy,
        pebbles_count: 0,
        max_pebbles_per_turn: 0,
    };
    let _res = program.send(PLAYER, invalid_init);

    // Valid initialization
    let game = init_game(&sys, DifficultyLevel::Easy, 15, 2);

    // Invalid turn operation
    let _res = game.send(PLAYER, PebblesAction::Turn(3));
}

#[test]
fn give_up_and_restart() {
    let sys = System::new();
    let game = init_game(&sys, DifficultyLevel::Easy, 15, 2);

    // Player's surrender
    let _res = game.send(PLAYER, PebblesAction::GiveUp);
    sys.run_next_block();

    let state: GameState = game.read_state(b"").unwrap();
    assert!(matches!(state.winner, Some(Player::Program)));

    // Restart game
    let _res = game.send(
        PLAYER,
        PebblesAction::Restart {
            difficulty: DifficultyLevel::Hard,
            pebbles_count: 20,
            max_pebbles_per_turn: 3,
        },
    );
    sys.run_next_block();

    let new_state: GameState = game.read_state(b"").unwrap();
    assert_eq!(new_state.pebbles_count, 20);
    assert_eq!(new_state.max_pebbles_per_turn, 3);
    assert!(matches!(new_state.difficulty, DifficultyLevel::Hard));
    assert!(new_state.winner.is_none());
}
