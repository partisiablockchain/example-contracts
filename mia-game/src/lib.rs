#![doc = include_str!("../README.md")]
#![allow(unused_variables)]

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;
extern crate pbc_lib;

mod zk_compute;

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::Address;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::sorted_vec_map::{SortedVecMap, SortedVecSet};
use pbc_contract_common::zk::{SecretVarId, ZkInputDef, ZkState, ZkStateChange};
use pbc_traits::ReadWriteState;
use pbc_zk::{Sbi8, SecretBinary};
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

/**
 * Metadata information associated with each individual variable.
 */
#[derive(ReadWriteState, ReadWriteRPC, Debug)]
#[repr(u8)]
pub enum SecretVarType {
    #[discriminant(0)]
    /// Randomness used for dice throws.
    Randomness {},
    #[discriminant(1)]
    /// Result of dice throws.
    ThrowResult {},
}

/// The state of the Mia game, which is persisted on-chain.
#[state]
pub struct MiaState {
    // The current amount of randomness contributions received for a dice throw.
    nr_of_randomness_contributions: u32,
    // The number of players at the start of the game.
    nr_of_players_at_the_start: u32,
    // The player currently throwing the dice and declaring a value.
    player_throwing: u32,
    // The current phase the game is in, to determine allowed actions.
    game_phase: GamePhase,
    // The players at the start of the game.
    starting_players: Vec<Address>,
    // The current players active in the game.
    players: Vec<Address>,
    // The remaining lives of the players in the game.
    player_lives: SortedVecMap<Address, u8>,
    // The last throw's secret variable id.
    throw_result_id: Option<SecretVarId>,
    // The stated value of the current throw.
    stated_throw: Option<DiceThrow>,
    // The revealed value of a throw.
    throw_result: Option<DiceThrow>,
    // The announced throw value, where the next announced throw must be higher than, to be eligible.
    throw_to_beat: DiceThrow,
    // The winner of the game.
    winner: Option<Address>,
}

impl MiaState {
    /// Get the current player in turn.
    fn current_player(&self) -> &Address {
        &self.players[self.player_throwing as usize]
    }
    /// Get the next player in turn.
    fn next_player(&self) -> &Address {
        &self.players[(self.player_throwing + 1) as usize % self.players.len()]
    }

    /// Replace the current player in turn with the next player.
    fn go_to_next_player(&mut self) {
        self.player_throwing = (self.player_throwing + 1) % self.players.len() as u32;
    }

    /// Check whether a player is dead i.e. have no lives left.
    fn is_player_dead(&self, player: Address) -> bool {
        self.player_lives[&player] == 0
    }

    /// Remove a dead player from the list of players.
    fn remove_dead_player(&mut self, player: Address) {
        self.players.retain(|p| player != *p);
    }

    /// Reduce a players lives by a given integer.
    fn reduce_players_life_by(&mut self, player: Address, lives_lost: u8) {
        if self.player_lives[&player] >= lives_lost {
            self.player_lives
                .insert(player, self.player_lives[&player] - lives_lost);
        } else {
            self.player_lives.insert(player, 0);
        }
    }

    /// Check whether the game if finished, i.e. whether only one player remains.
    fn is_the_game_finished(&self) -> bool {
        self.players.len() == 1
    }

    /// Get the last remaining player, the winner.
    fn get_winner(&self) -> Address {
        *self.players.get(0).unwrap()
    }
}

/// A throw of two dice.
#[derive(ReadWriteState, ReadWriteRPC, CreateTypeSpec, Debug, Copy, Clone)]
pub struct DiceThrow {
    d1: u8,
    d2: u8,
}

impl DiceThrow {
    /// The value of each die is reduced to be between 0 and 5.
    fn reduce(&self) -> DiceThrow {
        DiceThrow {
            d1: self.d1 % 6,
            d2: self.d2 % 6,
        }
    }

    /// Checks whether a throw is better than the current dice throw to beat.
    /// The dice throws are compared based on their associated values.
    fn better_than_or_equal(self, actual: DiceThrow) -> bool {
        self.get_throw_score() >= actual.get_throw_score()
    }

    /// Checks whether a dice throw is Mia, i.e. is (0,1) or (1,0).
    fn is_mia(self) -> bool {
        (self.d1 == 0 && self.d2 == 1) || (self.d2 == 0 && self.d1 == 1)
    }

    /// Checks whether a dice throw is Little Mia, i.e. is (0,2) or (2,0).
    fn is_little_mia(self) -> bool {
        (self.d1 == 0 && self.d2 == 2) || (self.d2 == 0 && self.d1 == 2)
    }

    /// Checks whether both dices in the dice throw have the same value.
    fn is_pair(self) -> bool {
        self.d1 == self.d2
    }

    /// Get the score of a dice throw.
    /// The throw values are determined such that the highest roll is Mia, then Little Mia,
    /// followed by the doubles from (5,5) to (0,0), and then all other rolls from (5,4)
    /// down to (2,1).
    fn get_throw_score(self) -> u8 {
        let mut value = 0;
        if self.is_mia() {
            value += 128;
        };
        if self.is_little_mia() {
            value += 64;
        };
        if self.is_pair() {
            value += 32;
        };
        if (self.d1 == 5) || (self.d2 == 5) {
            value += 16;
        }
        if (self.d1 == 4) || (self.d2 == 4) {
            value += 8;
        }
        if (self.d1 == 3) || (self.d2 == 3) {
            value += 4;
        }
        if (self.d1 == 2) || (self.d2 == 2) {
            value += 2;
        }
        if (self.d1 == 1) || (self.d2 == 1) {
            value += 1;
        }
        value
    }
}

/// The contribution each player must send to make a dice throw. The contributions should be in the
/// interval \[ 0, 5 \] inclusive. If the contributions are outside this interval,
/// they are normalized to the interval.
#[derive(CreateTypeSpec, SecretBinary)]
pub struct RandomContribution {
    d1: Sbi8,
    d2: Sbi8,
}

/// The different phases the contract can be in before, during and after a game of Mia.
#[derive(ReadWriteRPC, ReadWriteState, CreateTypeSpec, Debug, PartialEq, Copy, Clone)]
pub enum GamePhase {
    #[discriminant(0)]
    /// The game has been initialized.
    Start {},
    #[discriminant(1)]
    /// Players can add randomness as secret inputs.
    AddRandomness {},
    #[discriminant(2)]
    /// The player in turn can throw the dice.
    Throw {},
    #[discriminant(3)]
    /// The player in turn can announce their throw.
    Announce {},
    #[discriminant(4)]
    /// The next player in turn can believe or call out the player's announced throw.
    Decide {},
    #[discriminant(5)]
    /// If a player was called out, they can reveal their actual throw.
    Reveal {},
    #[discriminant(6)]
    /// The game is finished.
    Done {},
}

/// Initialize a new mia game.
///
/// # Arguments
///
/// * `_ctx` - the contract context containing information about the sender and the blockchain.
/// *
///
/// # Returns
///
/// The initial state of the petition, with no signers.
///
#[init(zk = true)]
pub fn initialize(
    context: ContractContext,
    zk_state: ZkState<SecretVarType>,
    addresses_to_play: Vec<Address>,
) -> (MiaState, Vec<EventGroup>) {
    assert!(
        addresses_to_play.len() >= 3,
        "There must be at least 3 players to play Mia."
    );
    assert_eq!(
        SortedVecSet::from(addresses_to_play.clone()).len(),
        addresses_to_play.len(),
        "No duplicates in players."
    );

    let mut state = MiaState {
        starting_players: addresses_to_play.clone(),
        players: addresses_to_play.clone(),
        nr_of_players_at_the_start: addresses_to_play.len() as u32,
        player_lives: SortedVecMap::new(),
        game_phase: GamePhase::Start {},
        player_throwing: 0,
        nr_of_randomness_contributions: 0,
        throw_result_id: None,
        stated_throw: None,
        throw_result: None,
        winner: None,
        throw_to_beat: DiceThrow { d1: 1, d2: 2 },
    };

    for address in addresses_to_play {
        state.player_lives.insert(address, 6);
    }

    (state, vec![])
}

/// Start the game.
///
/// # Arguments
///
/// * `ctx` - the contract context containing information about the sender and the blockchain.
/// * `state` - the current state of the game.
///
/// # Returns
///
/// The updated vote state reflecting the new signing.
///
#[action(shortname = 0x01, zk = true)]
pub fn start_round(
    context: ContractContext,
    mut state: MiaState,
    zk_state: ZkState<SecretVarType>,
) -> (MiaState, Vec<EventGroup>, Vec<ZkStateChange>) {
    assert_eq!(
        state.players[state.player_throwing as usize], context.sender,
        "Only the player whose turn it is can start the round."
    );
    state.game_phase = GamePhase::AddRandomness {};

    (state, vec![], vec![])
}

/// Add randomness for the next dice throw.
/// The sender must be a player in the game to add randomness.
#[zk_on_secret_input(shortname = 0x40, secret_type = "RandomContribution")]
pub fn add_randomness_to_throw(
    context: ContractContext,
    state: MiaState,
    zk_state: ZkState<SecretVarType>,
) -> (
    MiaState,
    Vec<EventGroup>,
    ZkInputDef<SecretVarType, RandomContribution>,
) {
    assert_eq!(
        state.game_phase,
        GamePhase::AddRandomness {},
        "Must be in the AddRandomness phase to input secret randomness."
    );
    assert!(state.starting_players.contains(&context.sender));
    assert!(
        zk_state
            .secret_variables
            .iter()
            .chain(zk_state.pending_inputs.iter())
            .all(|(_, secret_variable)| secret_variable.owner != context.sender),
        "Each Player is only allowed to send one contribution to the randomness of the dice throw. Sender: {:?}",
        context.sender
    );

    let input_def = ZkInputDef::with_metadata(SecretVarType::Randomness {});

    (state, vec![], input_def)
}

/// Automatically called when a variable is confirmed on chain.
///
/// Initializes opening.
#[zk_on_variable_inputted]
fn inputted_variable(
    context: ContractContext,
    mut state: MiaState,
    zk_state: ZkState<SecretVarType>,
    variable_id: SecretVarId,
) -> MiaState {
    if state.nr_of_randomness_contributions == state.nr_of_players_at_the_start - 1 {
        state.nr_of_randomness_contributions = 0;
        state.game_phase = GamePhase::Throw {};
    } else {
        state.nr_of_randomness_contributions += 1;
    }
    state
}

/// Start the computation to compute the dice throw.
#[action(shortname = 0x02, zk = true)]
pub fn throw_dice(
    context: ContractContext,
    state: MiaState,
    zk_state: ZkState<SecretVarType>,
) -> (MiaState, Vec<EventGroup>, Vec<ZkStateChange>) {
    assert_eq!(
        state.game_phase,
        GamePhase::Throw {},
        "The dice can only be thrown in the Throw phase"
    );
    assert_eq!(
        *state.current_player(),
        context.sender,
        "Only the player in turn can throw the dice. It is currently {:?}s turn",
        state.current_player()
    );

    (
        state,
        vec![],
        vec![zk_compute::compute_dice_throw_start(
            &SecretVarType::ThrowResult {},
        )],
    )
}

/// Automatically called when the sum of the random contributions are done.
/// Transfers the resulting throw to the player throwing the dice.
#[zk_on_compute_complete]
fn sum_compute_complete(
    _context: ContractContext,
    mut state: MiaState,
    zk_state: ZkState<SecretVarType>,
    output_variables: Vec<SecretVarId>,
) -> (MiaState, Vec<EventGroup>, Vec<ZkStateChange>) {
    let Some(result_id) = output_variables.get(0) else {
        panic!("No result")
    };

    state.throw_result_id = Some(*result_id);
    state.game_phase = GamePhase::Announce {};
    let player_to_transfer_to = *state.current_player();

    (
        state,
        vec![],
        vec![
            ZkStateChange::TransferVariable {
                variable: *result_id,
                new_owner: player_to_transfer_to,
            },
            ZkStateChange::DeleteVariables {
                variables_to_delete: zk_state
                    .secret_variables
                    .iter()
                    .map(|(variable_id, _)| variable_id)
                    .filter(|id| id != result_id)
                    .collect(),
            },
        ],
    )
}

/// Announce a value such that the next player can decide if they believe it or not.
/// The value must be higher than or equal to the throw to beat.
#[action(shortname = 0x03, zk = true)]
fn announce_throw(
    context: ContractContext,
    mut state: MiaState,
    zk_state: ZkState<SecretVarType>,
    dice_value: DiceThrow,
) -> (MiaState, Vec<EventGroup>, Vec<ZkStateChange>) {
    assert_eq!(
        *state.current_player(),
        context.sender,
        "Only the current player can state the value of the dice throw."
    );

    let reduced_dice_value = dice_value.reduce();

    if !reduced_dice_value.better_than_or_equal(state.throw_to_beat) {
        panic!("Stated throw must be better than the last stated throw.")
    }

    state.stated_throw = Some(dice_value);
    state.game_phase = GamePhase::Decide {};

    (state, vec![], vec![])
}

/// The next player believes the stated throw, and continues the round, where the throw to beat is
/// the stated throw.
#[action(shortname = 0x04, zk = true)]
fn believe(
    context: ContractContext,
    mut state: MiaState,
    zk_state: ZkState<SecretVarType>,
) -> (MiaState, Vec<EventGroup>, Vec<ZkStateChange>) {
    assert_eq!(
        context.sender,
        *state.next_player(),
        "Only the next player can say if they believe the stated throw."
    );
    assert_eq!(
        state.game_phase,
        GamePhase::Decide {},
        "Must be in the deciding phase to say believe."
    );

    state.game_phase = GamePhase::AddRandomness {};
    state.throw_to_beat = state.stated_throw.unwrap();
    state.stated_throw = None;
    state.go_to_next_player();

    (
        state,
        vec![],
        vec![ZkStateChange::DeleteVariables {
            variables_to_delete: zk_state
                .secret_variables
                .iter()
                .map(|(variable_id, _)| variable_id)
                .collect(),
        }],
    )
}

/// The next player does not believe the stated throw, and starts a reveal of the dice.
#[action(shortname = 0x05, zk = true)]
fn call_out(
    context: ContractContext,
    mut state: MiaState,
    zk_state: ZkState<SecretVarType>,
) -> (MiaState, Vec<EventGroup>, Vec<ZkStateChange>) {
    assert_eq!(
        context.sender,
        *state.next_player(),
        "Only the next player can say if the throwing player is lying."
    );
    assert_eq!(
        state.game_phase,
        GamePhase::Decide {},
        "Must be in the deciding phase say if the throwing player is lying."
    );
    let variable_to_open = state.throw_result_id.unwrap();
    state.game_phase = GamePhase::Reveal {};
    (
        state,
        vec![],
        vec![ZkStateChange::OpenVariables {
            variables: vec![variable_to_open],
        }],
    )
}

/// Saves the opened variable in state and readies another computation.
#[zk_on_variables_opened]
fn save_opened_variable(
    context: ContractContext,
    mut state: MiaState,
    zk_state: ZkState<SecretVarType>,
    opened_variables: Vec<SecretVarId>,
) -> (MiaState, Vec<EventGroup>, Vec<ZkStateChange>) {
    assert_eq!(
        opened_variables.len(),
        1,
        "Can only show one set of dice at a time."
    );

    let variable_id = opened_variables.get(0).unwrap();
    let result: DiceThrow = read_opened_variable_data(&zk_state, variable_id).unwrap();

    let result_reduced = result.reduce();

    let Some(stated_throw) = state.stated_throw else {
        panic!("Could not find a stated throw in state.")
    };

    let stated_throw_reduced = stated_throw.reduce();

    let loser_of_round = if result.better_than_or_equal(stated_throw_reduced) {
        *state.next_player()
    } else {
        *state.current_player()
    };

    if stated_throw.is_mia() {
        state.reduce_players_life_by(loser_of_round, 2);
    } else {
        state.reduce_players_life_by(loser_of_round, 1);
    }

    if state.is_player_dead(loser_of_round) {
        state.remove_dead_player(loser_of_round);
    }

    state.throw_result = Some(result_reduced);

    if state.is_the_game_finished() {
        state.game_phase = GamePhase::Done {};
        state.winner = Some(state.get_winner());
    } else {
        state.go_to_next_player();
        state.game_phase = GamePhase::AddRandomness {};
    }

    (
        state,
        vec![],
        vec![ZkStateChange::DeleteVariables {
            variables_to_delete: opened_variables,
        }],
    )
}

/// Reads the data from a revealed secret variable
fn read_opened_variable_data<T: ReadWriteState>(
    zk_state: &ZkState<SecretVarType>,
    variable_id: &SecretVarId,
) -> Option<T> {
    let variable = zk_state.get_variable(*variable_id)?;
    variable.open_value()
}
