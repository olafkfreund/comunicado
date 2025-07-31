/// Update function for TEA pattern
/// 
/// Central update function that handles all messages and updates the model accordingly.
/// This is the heart of the TEA architecture where all state changes happen.

use crate::tea::{Message, Model, Command, UpdateResult};
use crate::tea::message::{
    SystemMessage, UIMessage, EmailMessage, CalendarMessage, ContactsMessage, 
    AccountMessage, BackgroundMessage, NotificationMessage, ViewMode, ToastLevel,
    ToggleTarget, CalendarView
};
use crate::tea::model::{Toast, PhaseStatus, ComposeState, ComposeField, TaskState, TaskStatus};
use tokio::time::{Duration, Instant};
use uuid::Uuid;
use chrono::Datelike;

/// Main update function that processes messages and returns updated model with commands
pub fn update(model: Model, message: Message) -> UpdateResult<Model> {
    match message {
        Message::System(msg) => update_system(model, msg),
        Message::UI(msg) => update_ui(model, msg),
        Message::Email(msg) => update_email(model, msg),
        Message::Calendar(msg) => update_calendar(model, msg),
        Message::Contacts(msg) => update_contacts(model, msg),
        Message::Account(msg) => update_account(model, msg),
        Message::Background(msg) => update_background(model, msg),
        Message::Notification(msg) => update_notification(model, msg),
    }
}

/// Handle system messages
fn update_system(mut model: Model, message: SystemMessage) -> UpdateResult<Model> {
    match message {
        SystemMessage::Quit => {
            model.app_state.should_quit = true;
            UpdateResult::just_model(model)
        }
        
        SystemMessage::Initialize(startup_mode) => {
            // Set view based on startup mode
            model.current_view = match startup_mode {
                crate::cli::StartupMode::Default => ViewMode::Email,
                crate::cli::StartupMode::Email => ViewMode::Email,
                crate::cli::StartupMode::Calendar => ViewMode::Calendar,
                crate::cli::StartupMode::Contacts => ViewMode::Contacts,
            };
            
            // Start initialization
            model.app_state.initialization.in_progress = true;
            model.app_state.initialization.current_phase = Some("Database".to_string());
            model.app_state.initialization.phases.insert(
                "Database".to_string(), 
                PhaseStatus::InProgress
            );
            
            let commands = vec![
                Command::database(crate::tea::command::DatabaseCommand::Initialize),
            ];
            
            UpdateResult::new(model, commands)
        }
        
        SystemMessage::InitializationComplete => {
            model.app_state.initialization.complete = true;
            model.app_state.initialization.in_progress = false;
            model.app_state.initialization.current_phase = None;
            
            // Load initial data based on current view
            let commands = match model.current_view {
                ViewMode::Email => vec![
                    Command::message(Message::Email(EmailMessage::LoadMessages("INBOX".to_string()))),
                    Command::message(Message::Account(AccountMessage::LoadAccounts)),
                ],
                ViewMode::Calendar => vec![
                    Command::message(Message::Calendar(CalendarMessage::LoadEvents(
                        chrono::Local::now().date_naive(),
                        chrono::Local::now().date_naive() + chrono::Duration::days(30)
                    ))),
                ],
                ViewMode::Contacts => vec![
                    Command::message(Message::Contacts(ContactsMessage::LoadContacts)),
                ],
                ViewMode::Settings => vec![],
            };
            
            UpdateResult::new(model, commands)
        }
        
        SystemMessage::InitializationFailed(error) => {
            model.app_state.initialization.in_progress = false;
            model.app_state.initialization.error = Some(error.clone());
            
            if let Some(phase) = &model.app_state.initialization.current_phase {
                model.app_state.initialization.phases.insert(
                    phase.clone(), 
                    PhaseStatus::Failed(error.clone())
                );
            }
            
            let commands = vec![
                Command::ui(crate::tea::command::UICommand::ShowToast(
                    format!("Initialization failed: {}", error),
                    ToastLevel::Error,
                )),
            ];
            
            UpdateResult::new(model, commands)
        }
        
        SystemMessage::Resize(width, height) => {
            model.app_state.terminal_size = (width, height);
            UpdateResult::just_model(model)
        }
        
        SystemMessage::Tick => {
            model.app_state.last_tick = Instant::now();
            
            // Check for auto-sync
            let mut commands = Vec::new();
            if model.auto_sync.enabled && 
               model.app_state.last_tick.duration_since(model.auto_sync.last_sync) >= model.auto_sync.interval {
                commands.push(Command::message(Message::System(SystemMessage::AutoSync)));
            }
            
            // Clean up expired toasts
            let now = Instant::now();
            model.ui_state.toasts.retain(|toast| {
                now.duration_since(toast.created_at) < toast.duration
            });
            
            UpdateResult::new(model, commands)
        }
        
        SystemMessage::AutoSync => {
            model.auto_sync.last_sync = Instant::now();
            model.auto_sync.status = crate::tea::message::SyncStatus::Syncing;
            
            let commands = vec![
                Command::message(Message::Email(EmailMessage::SyncAll)),
                Command::message(Message::Calendar(CalendarMessage::SyncCalendar)),
                Command::message(Message::Contacts(ContactsMessage::SyncContacts)),
            ];
            
            UpdateResult::new(model, commands)
        }
    }
}

/// Handle UI messages
fn update_ui(mut model: Model, message: UIMessage) -> UpdateResult<Model> {
    match message {
        UIMessage::KeyPressed(_key_event) => {
            // TODO: Handle keyboard shortcuts based on current view and context
            UpdateResult::just_model(model)
        }
        
        UIMessage::MouseEvent(_mouse_event) => {
            // TODO: Handle mouse events
            UpdateResult::just_model(model)
        }
        
        UIMessage::Navigate(view_mode) => {
            model.current_view = view_mode;
            
            // Load data for the new view if needed
            let commands = match view_mode {
                ViewMode::Email if model.email_state.messages.is_empty() => {
                    vec![Command::message(Message::Email(EmailMessage::LoadMessages("INBOX".to_string())))]
                }
                ViewMode::Calendar if model.calendar_state.events.is_empty() => {
                    vec![Command::message(Message::Calendar(CalendarMessage::LoadEvents(
                        chrono::Local::now().date_naive(),
                        chrono::Local::now().date_naive() + chrono::Duration::days(30)
                    )))]
                }
                ViewMode::Contacts if model.contacts_state.contacts.is_empty() => {
                    vec![Command::message(Message::Contacts(ContactsMessage::LoadContacts))]
                }
                _ => vec![],
            };
            
            UpdateResult::new(model, commands)
        }
        
        UIMessage::Toggle(target) => {
            match target {
                ToggleTarget::Sidebar => {
                    model.ui_state.sidebar_visible = !model.ui_state.sidebar_visible;
                }
                ToggleTarget::StatusBar => {
                    model.ui_state.status_bar_visible = !model.ui_state.status_bar_visible;
                }
                ToggleTarget::HelpOverlay => {
                    model.ui_state.help_visible = !model.ui_state.help_visible;
                }
                ToggleTarget::SearchBar => {
                    model.ui_state.search.active = !model.ui_state.search.active;
                    if !model.ui_state.search.active {
                        model.ui_state.search.query.clear();
                        model.ui_state.search.results_count = None;
                    }
                }
                ToggleTarget::FilterPanel => {
                    // TODO: Implement filter panel toggle
                }
            }
            UpdateResult::just_model(model)
        }
        
        UIMessage::ToggleHelp => {
            model.ui_state.help_visible = !model.ui_state.help_visible;
            UpdateResult::just_model(model)
        }
        
        UIMessage::ShowContextMenu(_menu_type) => {
            // TODO: Implement context menu display
            UpdateResult::just_model(model)
        }
        
        UIMessage::HideContextMenu => {
            model.ui_state.context_menu = None;
            UpdateResult::just_model(model)
        }
        
        UIMessage::ShowToast(message, level) => {
            let toast = Toast {
                id: Uuid::new_v4().to_string(),
                message,
                level,
                created_at: Instant::now(),
                duration: match level {
                    ToastLevel::Info => Duration::from_secs(3),
                    ToastLevel::Success => Duration::from_secs(2),
                    ToastLevel::Warning => Duration::from_secs(4),
                    ToastLevel::Error => Duration::from_secs(5),
                },
            };
            
            model.ui_state.toasts.push(toast);
            UpdateResult::just_model(model)
        }
        
        UIMessage::SearchChanged(query) => {
            model.ui_state.search.query = query;
            model.ui_state.search.loading = true;
            
            // Trigger search based on current view
            let commands = match model.current_view {
                ViewMode::Email => vec![
                    Command::message(Message::Email(EmailMessage::Search(model.ui_state.search.query.clone())))
                ],
                ViewMode::Contacts => vec![
                    Command::message(Message::Contacts(ContactsMessage::Search(model.ui_state.search.query.clone())))
                ],
                _ => vec![],
            };
            
            UpdateResult::new(model, commands)
        }
        
        UIMessage::SearchSubmit => {
            // Search already triggered by SearchChanged
            UpdateResult::just_model(model)
        }
        
        UIMessage::SearchClear => {
            model.ui_state.search.query.clear();
            model.ui_state.search.results_count = None;
            model.ui_state.search.loading = false;
            
            // Reload default view data
            let commands = match model.current_view {
                ViewMode::Email => vec![
                    Command::message(Message::Email(EmailMessage::LoadMessages(
                        model.email_state.current_folder.clone().unwrap_or_else(|| "INBOX".to_string())
                    )))
                ],
                ViewMode::Contacts => vec![
                    Command::message(Message::Contacts(ContactsMessage::LoadContacts))
                ],
                _ => vec![],
            };
            
            UpdateResult::new(model, commands)
        }
    }
}

/// Handle email messages
fn update_email(mut model: Model, message: EmailMessage) -> UpdateResult<Model> {
    match message {
        EmailMessage::LoadMessages(folder_name) => {
            model.email_state.current_folder = Some(folder_name.clone());
            model.email_state.loading = true;
            
            let commands = vec![
                Command::database(crate::tea::command::DatabaseCommand::LoadMessages(folder_name)),
            ];
            
            UpdateResult::new(model, commands)
        }
        
        EmailMessage::MessagesLoaded(_messages) => {
            // Convert from crate::email::EmailMessage to model storage
            // For now, we'll store them as-is but in a real implementation
            // you'd convert to a unified message format
            model.email_state.loading = false;
            model.email_state.last_sync = Some(chrono::Local::now());
            model.email_state.sync_status = crate::tea::message::SyncStatus::Success;
            UpdateResult::just_model(model)
        }
        
        EmailMessage::LoadingFailed(error) => {
            model.email_state.loading = false;
            model.email_state.sync_status = crate::tea::message::SyncStatus::Error;
            
            let commands = vec![
                Command::ui(crate::tea::command::UICommand::ShowToast(
                    format!("Failed to load messages: {}", error),
                    ToastLevel::Error,
                )),
            ];
            
            UpdateResult::new(model, commands)
        }
        
        EmailMessage::SelectMessage(message_id) => {
            model.email_state.selected_message = Some(message_id);
            UpdateResult::just_model(model)
        }
        
        EmailMessage::OpenMessage(message_id) => {
            model.email_state.reading_message = Some(message_id);
            UpdateResult::just_model(model)
        }
        
        EmailMessage::ComposeNew => {
            model.email_state.compose = Some(ComposeState {
                to: String::new(),
                cc: String::new(),
                bcc: String::new(),
                subject: String::new(),
                body: String::new(),
                in_reply_to: None,
                attachments: Vec::new(),
                current_field: ComposeField::To,
            });
            UpdateResult::just_model(model)
        }
        
        EmailMessage::Reply(message_id) => {
            // TODO: Pre-fill compose state with reply data
            model.email_state.compose = Some(ComposeState {
                to: String::new(), // TODO: Extract from original message
                cc: String::new(),
                bcc: String::new(),
                subject: String::new(), // TODO: Add "Re: " prefix
                body: String::new(), // TODO: Quote original message
                in_reply_to: Some(message_id),
                attachments: Vec::new(),
                current_field: ComposeField::Body,
            });
            UpdateResult::just_model(model)
        }
        
        EmailMessage::Forward(_message_id) => {
            // TODO: Pre-fill compose state with forward data
            model.email_state.compose = Some(ComposeState {
                to: String::new(),
                cc: String::new(),
                bcc: String::new(),
                subject: String::new(), // TODO: Add "Fwd: " prefix
                body: String::new(), // TODO: Include original message
                in_reply_to: None,
                attachments: Vec::new(), // TODO: Include original attachments
                current_field: ComposeField::To,
            });
            UpdateResult::just_model(model)
        }
        
        EmailMessage::Delete(message_id) => {
            let commands = vec![
                Command::database(crate::tea::command::DatabaseCommand::DeleteMessage(message_id)),
            ];
            UpdateResult::new(model, commands)
        }
        
        EmailMessage::ToggleRead(message_id) => {
            let commands = vec![
                Command::database(crate::tea::command::DatabaseCommand::UpdateMessageFlags(
                    message_id, 
                    vec!["\\Seen".to_string()]
                )),
            ];
            UpdateResult::new(model, commands)
        }
        
        EmailMessage::ToggleFlag(message_id) => {
            let commands = vec![
                Command::database(crate::tea::command::DatabaseCommand::UpdateMessageFlags(
                    message_id, 
                    vec!["\\Flagged".to_string()]
                )),
            ];
            UpdateResult::new(model, commands)
        }
        
        EmailMessage::MoveToFolder(_message_id, _folder_name) => {
            // TODO: Implement message move
            UpdateResult::just_model(model)
        }
        
        EmailMessage::SyncFolder(folder_name) => {
            let commands = vec![
                Command::network(crate::tea::command::NetworkCommand::SyncIMAPFolder(
                    model.account_state.active_account.clone().unwrap_or_default(),
                    folder_name,
                )),
            ];
            UpdateResult::new(model, commands)
        }
        
        EmailMessage::SyncAll => {
            model.email_state.sync_status = crate::tea::message::SyncStatus::Syncing;
            
            let commands = if let Some(account_id) = &model.account_state.active_account {
                vec![
                    Command::network(crate::tea::command::NetworkCommand::ConnectIMAP(account_id.clone())),
                ]
            } else {
                vec![
                    Command::ui(crate::tea::command::UICommand::ShowToast(
                        "No active account for sync".to_string(),
                        ToastLevel::Warning,
                    )),
                ]
            };
            
            UpdateResult::new(model, commands)
        }
        
        EmailMessage::Search(_query) => {
            model.ui_state.search.loading = true;
            // TODO: Implement email search
            UpdateResult::just_model(model)
        }
    }
}

/// Handle calendar messages
fn update_calendar(mut model: Model, message: CalendarMessage) -> UpdateResult<Model> {
    match message {
        CalendarMessage::LoadEvents(start_date, end_date) => {
            model.calendar_state.loading = true;
            
            let commands = vec![
                Command::database(crate::tea::command::DatabaseCommand::LoadEvents(start_date, end_date)),
            ];
            
            UpdateResult::new(model, commands)
        }
        
        CalendarMessage::EventsLoaded(events) => {
            model.calendar_state.events = events;
            model.calendar_state.loading = false;
            model.calendar_state.last_sync = Some(chrono::Local::now());
            model.calendar_state.sync_status = crate::tea::message::SyncStatus::Success;
            UpdateResult::just_model(model)
        }
        
        CalendarMessage::LoadingFailed(error) => {
            model.calendar_state.loading = false;
            model.calendar_state.sync_status = crate::tea::message::SyncStatus::Error;
            
            let commands = vec![
                Command::ui(crate::tea::command::UICommand::ShowToast(
                    format!("Failed to load events: {}", error),
                    ToastLevel::Error,
                )),
            ];
            
            UpdateResult::new(model, commands)
        }
        
        CalendarMessage::SelectEvent(event_id) => {
            model.calendar_state.selected_event = Some(event_id);
            UpdateResult::just_model(model)
        }
        
        CalendarMessage::ChangeView(view) => {
            model.calendar_state.view = view;
            
            // Load events for new view if needed
            let (start_date, end_date) = match view {
                CalendarView::Day => {
                    let date = model.calendar_state.current_date;
                    (date, date)
                }
                CalendarView::Week => {
                    let date = model.calendar_state.current_date;
                    let start = date - chrono::Duration::days(date.weekday().num_days_from_monday() as i64);
                    let end = start + chrono::Duration::days(6);
                    (start, end)
                }
                CalendarView::Month => {
                    let date = model.calendar_state.current_date;
                    let start = date.with_day(1).unwrap();
                    let end = if date.month() == 12 {
                        chrono::NaiveDate::from_ymd_opt(date.year() + 1, 1, 1).unwrap() - chrono::Duration::days(1)
                    } else {
                        chrono::NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1).unwrap() - chrono::Duration::days(1)
                    };
                    (start, end)
                }
                CalendarView::Agenda => {
                    let date = model.calendar_state.current_date;
                    (date, date + chrono::Duration::days(30))
                }
            };
            
            let commands = vec![
                Command::message(Message::Calendar(CalendarMessage::LoadEvents(start_date, end_date))),
            ];
            
            UpdateResult::new(model, commands)
        }
        
        CalendarMessage::NavigateToDate(date) => {
            model.calendar_state.current_date = date;
            
            // Reload events for new date
            let commands = match model.calendar_state.view {
                CalendarView::Day => vec![
                    Command::message(Message::Calendar(CalendarMessage::LoadEvents(date, date))),
                ],
                _ => vec![], // Other views will handle date navigation differently
            };
            
            UpdateResult::new(model, commands)
        }
        
        CalendarMessage::SyncCalendar => {
            model.calendar_state.sync_status = crate::tea::message::SyncStatus::Syncing;
            
            let commands = vec![
                Command::network(crate::tea::command::NetworkCommand::SyncCalendar("default".to_string())),
            ];
            
            UpdateResult::new(model, commands)
        }
        
        _ => {
            // TODO: Implement remaining calendar message handlers
            UpdateResult::just_model(model)
        }
    }
}

/// Handle contacts messages
fn update_contacts(mut model: Model, message: ContactsMessage) -> UpdateResult<Model> {
    match message {
        ContactsMessage::LoadContacts => {
            model.contacts_state.loading = true;
            
            let commands = vec![
                Command::database(crate::tea::command::DatabaseCommand::LoadContacts),
            ];
            
            UpdateResult::new(model, commands)
        }
        
        ContactsMessage::ContactsLoaded(contacts) => {
            model.contacts_state.contacts = contacts;
            model.contacts_state.loading = false;
            model.contacts_state.last_sync = Some(chrono::Local::now());
            model.contacts_state.sync_status = crate::tea::message::SyncStatus::Success;
            UpdateResult::just_model(model)
        }
        
        ContactsMessage::LoadingFailed(error) => {
            model.contacts_state.loading = false;
            model.contacts_state.sync_status = crate::tea::message::SyncStatus::Error;
            
            let commands = vec![
                Command::ui(crate::tea::command::UICommand::ShowToast(
                    format!("Failed to load contacts: {}", error),
                    ToastLevel::Error,
                )),
            ];
            
            UpdateResult::new(model, commands)
        }
        
        ContactsMessage::SelectContact(contact_id) => {
            model.contacts_state.selected_contact = Some(contact_id);
            UpdateResult::just_model(model)
        }
        
        ContactsMessage::SyncContacts => {
            model.contacts_state.sync_status = crate::tea::message::SyncStatus::Syncing;
            
            let commands = vec![
                Command::network(crate::tea::command::NetworkCommand::SyncContacts(
                    model.account_state.active_account.clone().unwrap_or_default()
                )),
            ];
            
            UpdateResult::new(model, commands)
        }
        
        _ => {
            // TODO: Implement remaining contacts message handlers
            UpdateResult::just_model(model)
        }
    }
}

/// Handle account messages
fn update_account(mut model: Model, message: AccountMessage) -> UpdateResult<Model> {
    match message {
        AccountMessage::LoadAccounts => {
            model.account_state.loading = true;
            // TODO: Load accounts from storage
            UpdateResult::just_model(model)
        }
        
        AccountMessage::AccountsLoaded(accounts) => {
            model.account_state.accounts = accounts;
            model.account_state.loading = false;
            
            // Set first account as active if none selected
            if model.account_state.active_account.is_none() && !model.account_state.accounts.is_empty() {
                model.account_state.active_account = Some(model.account_state.accounts[0].account_id.clone());
            }
            
            UpdateResult::just_model(model)
        }
        
        AccountMessage::SyncStatusChanged(account_id, status) => {
            model.account_state.sync_status.insert(account_id, status);
            UpdateResult::just_model(model)
        }
        
        _ => {
            // TODO: Implement remaining account message handlers
            UpdateResult::just_model(model)
        }
    }
}

/// Handle background messages
fn update_background(mut model: Model, message: BackgroundMessage) -> UpdateResult<Model> {
    match message {
        BackgroundMessage::TaskStarted(task_id) => {
            model.background_state.tasks.insert(task_id.clone(), TaskState {
                name: task_id.clone(),
                started_at: Instant::now(),
                progress: None,
                status: TaskStatus::Running,
            });
            model.background_state.processing = true;
            UpdateResult::just_model(model)
        }
        
        BackgroundMessage::TaskCompleted(task_id) => {
            if let Some(task) = model.background_state.tasks.get_mut(&task_id) {
                task.status = TaskStatus::Completed;
            }
            
            // Check if all tasks are complete
            let all_complete = model.background_state.tasks.values().all(|task| {
                matches!(task.status, TaskStatus::Completed | TaskStatus::Failed(_))
            });
            
            if all_complete {
                model.background_state.processing = false;
            }
            
            UpdateResult::just_model(model)
        }
        
        BackgroundMessage::TaskFailed(task_id, error) => {
            if let Some(task) = model.background_state.tasks.get_mut(&task_id) {
                task.status = TaskStatus::Failed(error.clone());
            }
            
            let commands = vec![
                Command::ui(crate::tea::command::UICommand::ShowToast(
                    format!("Task failed: {}", error),
                    ToastLevel::Error,
                )),
            ];
            
            UpdateResult::new(model, commands)
        }
        
        BackgroundMessage::TaskProgress(task_id, current, total) => {
            if let Some(task) = model.background_state.tasks.get_mut(&task_id) {
                task.progress = Some((current, total));
            }
            UpdateResult::just_model(model)
        }
    }
}

/// Handle notification messages
fn update_notification(model: Model, message: NotificationMessage) -> UpdateResult<Model> {
    match message {
        NotificationMessage::NewEmail(sender, subject) => {
            let commands = vec![
                Command::ui(crate::tea::command::UICommand::ShowToast(
                    format!("New email from {}: {}", sender, subject),
                    ToastLevel::Info,
                )),
                Command::system(crate::tea::command::SystemCommand::ShowDesktopNotification(
                    "New Email".to_string(),
                    format!("From: {}\n{}", sender, subject),
                )),
            ];
            
            UpdateResult::new(model, commands)
        }
        
        NotificationMessage::CalendarReminder(event_title, time) => {
            let commands = vec![
                Command::ui(crate::tea::command::UICommand::ShowToast(
                    format!("Reminder: {} at {}", event_title, time),
                    ToastLevel::Warning,
                )),
                Command::system(crate::tea::command::SystemCommand::ShowDesktopNotification(
                    "Calendar Reminder".to_string(),
                    format!("{} at {}", event_title, time),
                )),
            ];
            
            UpdateResult::new(model, commands)
        }
        
        NotificationMessage::System(message) => {
            let commands = vec![
                Command::ui(crate::tea::command::UICommand::ShowToast(message, ToastLevel::Info)),
            ];
            
            UpdateResult::new(model, commands)
        }
        
        NotificationMessage::Error(message) => {
            let commands = vec![
                Command::ui(crate::tea::command::UICommand::ShowToast(message, ToastLevel::Error)),
            ];
            
            UpdateResult::new(model, commands)
        }
        
        NotificationMessage::Success(message) => {
            let commands = vec![
                Command::ui(crate::tea::command::UICommand::ShowToast(message, ToastLevel::Success)),
            ];
            
            UpdateResult::new(model, commands)
        }
    }
}