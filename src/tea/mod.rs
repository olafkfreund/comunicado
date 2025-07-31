/// The Elm Architecture (TEA) pattern implementation for Comunicado
/// 
/// This module provides a structured approach to state management following
/// the Model-Update-View pattern from Elm, adapted for Ratatui terminal interfaces.

pub mod message;
pub mod model;
pub mod update;
pub mod command;

pub use message::Message;
pub use model::Model;
pub use update::update;
pub use command::{Command, CommandExecutor};

/// Core TEA architecture trait for components
pub trait TeaComponent<Msg, Mdl> {
    /// Update the model based on a message
    fn update(&self, model: &mut Mdl, message: Msg) -> Vec<Command>;
    
    /// Subscribe to external events (optional)
    fn subscriptions(&self, _model: &Mdl) -> Vec<Command> {
        Vec::new()
    }
}

/// Result of processing a TEA update cycle
pub struct UpdateResult<M> {
    pub model: M,
    pub commands: Vec<Command>,
}

impl<M> UpdateResult<M> {
    pub fn new(model: M, commands: Vec<Command>) -> Self {
        Self { model, commands }
    }
    
    pub fn just_model(model: M) -> Self {
        Self { model, commands: Vec::new() }
    }
    
    pub fn with_command(model: M, command: Command) -> Self {
        Self { model, commands: vec![command] }
    }
}