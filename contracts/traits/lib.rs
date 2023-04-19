#![cfg_attr(not(feature = "std"), no_std)]

use ink::primitives::{AccountId, Hash};
use ink::prelude::vec::Vec;
use ink::primitives::AccountId;
use ink::storage::traits::StorageLayout;
use scale::{Decode, Encode};

// TODO: add some events

/// Game errors.
#[derive(Encode, Decode, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum GameError {
    /// Caller must match the palyer being added
    CallerMustMatchNewPlayer,
    /// No more space for players to join
    MaxPlayersReached,
    /// Fees paid to join the game are not sufficient
    InsufficientJoiningFees,
    /// The round has not expired yet
    RoundNotExpired,
    /// No commitment made by player for the current round
    CommitmentNotFound,
    /// Round cannot be closed
    FailedToCloseRound,
}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
pub enum GameStatus {
    Initialized,
    Ready,
    Started,
    Ended,
}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
pub enum RoundStatus {
    Ready,
    Started,
    PendingRewardsClaim,
    Ended,
}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
pub struct GameRound {
    pub round_number: u32,
    pub status: RoundStatus,
    pub player_commits: Vec<(AccountId, u128)>,
    pub player_reveals: Vec<(AccountId, u128)>,
    pub player_contributions: Vec<(AccountId, u128)>,
    pub total_contribution: u128,
    pub total_reward: u128,
}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
pub struct GameConfigs {
    pub max_players: u8,
    pub min_players: u8,
    pub min_round_contribution: Option<u128>,
    pub max_round_contribution: Option<u128>,
    pub post_round_actions: bool,
    /// The number of blocks before a round is considered stale.
    pub round_timeout: Option<u32>,
    pub max_rounds: Option<u32>,
    pub join_fee: Option<u128>,
}

/// Defines the basic game lifecycle methods.
#[ink::trait_definition]
pub trait GameLifecycle {
    /// Gets the AccountId of each each player within this instance of the game.
    #[ink(message)]
    fn get_configs(&self) -> GameConfigs;

    /// Gets the AccountId of each each player within this instance of the game.
    #[ink(message)]
    fn get_players(&self) -> Vec<AccountId>;

    /// Get the status of the current game.
    #[ink(message)]
    fn get_status(&self) -> GameStatus;

    /// Get the current game round.
    #[ink(message)]
    fn get_current_round(&self) -> Option<GameRound>;

    /// Adds a player into the game by their AccountId.
    /// Ensures that caller of the function has the same AccountId being added (i.e. player can add themselves).
    ///
    /// Returns the number of players.
    #[ink(message, payable)]
    fn join(&mut self, player: AccountId) -> Result<u8, GameError>;

    /// callable by any player who joined
    /// only works once, fails on subsequent calls (since the state has changed)
    /// emits a relevant event (all events should include some game ID for the UIs that are listening)
    #[ink(message, payable)]
    fn startGame(&mut self) -> Result<(), GameError>;

    /// Makes a commitment to the current round by the player who called the method
    /// The payed amount is the round contribution, to be validated based on configs
    /// Must be recorded in the GameRound storage
    /// emits a relevant event (should include the total # of commitments in the round, helps UI know if everyone played)
    #[ink(message, payable)]
    fn playRound(&mut self, commitment: Hash) -> Result<(), GameError>;

    /// receives data which if hashed must match the commitment for the round made earlier
    /// throws an error if the round has no commitment for the caller
    /// prepares the next round if max rounds not reached
    /// emits a relevant event
    #[ink(message, payable)]
    fn revealRound(&mut self, reveal: ([u8; 32], u8)) -> Result<(), GameError>;

    /// claims rewards of the round (if applicable and all players have revealed)
    /// prepares the next round
    /// emits a relevant event
    #[ink(message, payable)]
    fn completeRound(&mut self) -> Result<(), GameError>;

    /// succeeds only if the caller has already made a commitment
    /// succeeds only if the round expired (passed the block timeout in config // should default to 10 or 20 blocks if None)
    /// a penalty is incurred by the players who did not play (joining fee is not returned)
    /// emits a relevant event
    #[ink(message, payable)]
    fn forceCompleteRound(&mut self) -> Result<(), GameError>;
}
