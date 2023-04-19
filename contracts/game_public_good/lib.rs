#![cfg_attr(not(feature = "std"), no_std)]

pub use self::game_public_good::{GamePublicGood, GamePublicGoodRef};

#[ink::contract]
pub mod game_public_good {
    use traits::{ GameLifecycle, GameRound, GameStatus, GameConfigs, GameError, RoundStatus };
    use ink::prelude::vec::Vec;
    use ink::env::hash::{Blake2x256, HashOutput};

    /// A single game storage.
    /// Each contract (along with its storage) represents a single game instance.
    #[ink(storage)]
    pub struct GamePublicGood {
        /// Stores the list of players for this game instance
        players: Vec<AccountId>,
        /// The status of the current game
        status: GameStatus,
        /// A list of all the rounds that have been played
        rounds: Vec<GameRound>,
        /// The current round of the game
        current_round: Option<GameRound>,
        /// The id of the next round
        next_round_id: u8,
        /// The configurations of the game
        configs: GameConfigs,
    }

    impl GamePublicGood {
        /// Constructor that initializes the GamePublicGood struct
        #[ink(constructor)]
        pub fn new(configs: GameConfigs) -> Self {
            Self {
                players: Vec::new(),
                status: GameStatus::Initialized,
                rounds: Vec::new(),
                current_round: None,
                next_round_id: 1,
                configs,
            }
        }

        /// A default constructor that initializes this game with 10 players.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(GameConfigs {
                max_players: 10,
                min_players: 2,
                min_round_contribution: None,
                max_round_contribution: Some(1_000),
                post_round_actions: false,
                round_timeout: None,
                max_rounds: None,
                join_fee: None,
                is_rounds_based: false,
            })
        }
    }

    /// An implementation of the `GameLifecycle` trait for the `GamePublicGood` contract.
    impl GameLifecycle for GamePublicGood {
        #[ink(message)]
        fn get_configs(&self) -> GameConfigs {
            self.configs.clone()
        }

        #[ink(message)]
        fn get_players(&self) -> Vec<AccountId> {
            self.players.clone()
        }

        #[ink(message)]
        fn get_status(&self) -> GameStatus {
            self.status
        }

        #[ink(message)]
        fn get_current_round(&self) -> Option<GameRound> {
            self.current_round.clone()
        }

        #[ink(message, payable)]
        fn join(&mut self, player: AccountId) -> Result<u8, GameError> {
            if self.env().caller() != player {
                return Err(GameError::CallerMustMatchNewPlayer)
            }
            
            if self.players.len() >= self.configs.max_players as usize {
                return Err(GameError::MaxPlayersReached)
            }

            if let Some(fees) = self.configs.join_fee {
                if self.env().transferred_value() < fees {
                    return Err(GameError::InsufficientJoiningFees);
                }
            }

            self.players.push(player);
            Ok(self.players.len() as u8)
        }

        #[ink(message, payable)]
        fn start_game(&mut self) -> Result<(), GameError> {
            match (self.players.len(), self.status) {
                (_, status) if status != GameStatus::Initialized => {
                    return Err(GameError::InvalidGameStartState)
                },
                (players, _) if players < self.configs.min_players as usize => {
                    return Err(GameError::NotEnoughPlayers)
                },
                _ => (),
            }

            self.current_round = Some(GameRound {
                id: self.next_round_id,
                status: RoundStatus::Ready,
                player_commits: Vec::new(),
                player_reveals: Vec::new(),
                player_contributions: Vec::new(),
                total_contribution: 0,
                total_reward: 0,
            });
            self.status = GameStatus::Started;
            self.next_round_id += 1;
            Ok(())
        }

        #[ink(message, payable)]
        fn play_round(&mut self, commitment: Hash) -> Result<(), GameError> {
            match (self.status, self.current_round.is_none(), self.env().transferred_value()) {
                (status, _, _) if status != GameStatus::Started => {
                    return Err(GameError::GameNotStarted)
                },
                (_, true, _) => {
                    return Err(GameError::NoCurrentRound)
                },
                (_, _, value) if Some(value) < self.configs.max_round_contribution => {
                    // NOTE: the issue here is since this game is publicgood, some amount has to be
                    // contributed to the pot. So, we need to check if the player has contributed
                    // that amount. But we also don't want to reveal the contribution :)
                    // one way is to have the payable amount always be fixed and be maxed out
                    // while the hashed commitment contains the real amount to be contributed.
                    return Err(GameError::InvalidRoundContribution)
                },
                _ => ()
            }

            let caller = self.env().caller();
            let value = self.env().transferred_value();
            let current_round = self.current_round.as_mut().unwrap();

            // store the commit
            current_round.player_commits.push((
                caller.clone(),
                commitment,
            ));

            // keep track of round contribution(s)
            current_round.player_contributions.push((
                caller.clone(),
                value,
            ));

            current_round.total_contribution += value;

            // check if all players have committed
            if current_round.player_commits.len() == self.players.len() {
                // TODO: emit AllPlayersCommitted event
            }

            self.current_round = Some(current_round.clone());
            Ok(())
        }

        #[ink(message, payable)]
        fn reveal_round(&mut self, reveal: (u128, u128)) -> Result<(), GameError> {
            let caller = self.env().caller();
            let data = [reveal.0.to_le_bytes(), reveal.1.to_le_bytes()].concat();
            let mut output = <Blake2x256 as HashOutput>::Type::default();
            ink::env::hash_bytes::<Blake2x256>(&data, &mut output);

            let player_commitment = self.current_round
                .as_ref()
                .unwrap()
                .player_commits
                .iter()
                .find(|(player, _)| player == &caller);

            // check if the reveal is valid
            match player_commitment {
                Some((_, commitment)) => {
                    if commitment != &output.into() {
                        return Err(GameError::InvalidReveal)
                    }
                }
                None => return Err(GameError::CommitmentNotFound),
            }

            // store the reveal
            self.current_round.as_mut().unwrap().player_reveals.push((
                caller,
                reveal,
            ));

            // TODO: emit an event for the reveal

            Ok(())
        }

        #[ink(message, payable)]
        fn complete_round(&mut self) -> Result<(), GameError> {
            todo!("implement")
        }

        #[ink(message, payable)]
        fn force_complete_round(&mut self) -> Result<(), GameError> {
            todo!("implement")
        }

        #[ink(message, payable)]
        fn end_game(&mut self) -> Result<(), GameError> {
            todo!("implement")
        }
    }

    /// Unit tests.
    #[cfg(test)]
    mod tests {
        use super::*;

        /// Default constructor works.
        #[ink::test]
        fn default_works() {
            let game_public_good = GamePublicGood::default();
            assert_eq!(game_public_good.players, vec![]);
            assert_eq!(game_public_good.get_current_round(), None);
        }

        /// Can construct with "new()" method.
        #[ink::test]
        fn new_works() {
            let game_public_good = GamePublicGood::new(GameConfigs {
                max_players: 10,
                min_players: 2,
                min_round_contribution: None,
                max_round_contribution: None,
                post_round_actions: false,
                round_timeout: None,
                max_rounds: None,
                join_fee: None,
                is_rounds_based: false,
            });
            assert_eq!(game_public_good.players, vec![]);
            assert_eq!(game_public_good.get_current_round(), None);
        }

        /// A new player can join the game.
        #[ink::test]
        fn player_can_join() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            let mut game_public_good = GamePublicGood::default();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            // can join when the caller is alice joining as alice (own account)
            assert!(game_public_good.join(accounts.alice).is_ok());
        }

        /// A new player cannot add someone else to the game.
        #[ink::test]
        fn player_must_join_as_self() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            let mut game_public_good = GamePublicGood::default();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            // can't join when the caller is alice trying to add bob's account
            assert!(game_public_good.join(accounts.bob).is_err());
        }

        /// A player can start the game.
        #[ink::test]
        fn player_can_start_game() {
            let accounts = 
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            let mut game_public_good = GamePublicGood::default();
            
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            assert!(game_public_good.join(accounts.alice).is_ok());
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert!(game_public_good.join(accounts.bob).is_ok());
            
            // can start the game when there are enough players
            match game_public_good.start_game() {
                Err(error) => {
                    println!("{:?}", error);
                    assert!(false);
                },
                Ok(_) => assert!(true),
            }
        }

        /// A player cannot start a game that is already started.
        #[ink::test]
        fn player_cannot_start_already_started_game() {
            let accounts = 
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            let mut game_public_good = GamePublicGood::default();
            
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            assert!(game_public_good.join(accounts.alice).is_ok());
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert!(game_public_good.join(accounts.bob).is_ok());
            
            // can start the game when there are enough players
            assert!(game_public_good.start_game().is_ok());
            // cannot start again
            assert_eq!(game_public_good.start_game().err(), Some(GameError::InvalidGameStartState));
        }

        /// A player cannot start a game that doesn't have enough players.
        #[ink::test]
        fn game_cannot_start_without_enough_players() {
            let accounts = 
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            let mut game_public_good = GamePublicGood::default();
            
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            assert!(game_public_good.join(accounts.alice).is_ok());
            
            // cannot start, not enough players
            assert_eq!(game_public_good.start_game().err(), Some(GameError::NotEnoughPlayers));
        }
    }

    /// On-chain (E2E) tests.
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        use super::*;
        use ink_e2e::build_message;
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        // Default constructor works.
        #[ink_e2e::test]
        async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            let constructor = GamePublicGoodRef::default();

            // When
            let contract_account_id = client
                .instantiate("game_public_good", &ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiation failed")
                .account_id;

            // Then
            let get_players = build_message::<GamePublicGoodRef>(contract_account_id.clone())
                .call(|test| test.get_players());
            let get_result = client
                .call_dry_run(&ink_e2e::alice(), &get_players, 0, None)
                .await;
            assert_eq!(get_result.return_value(), vec![]);

            Ok(())
        }
    }
}
