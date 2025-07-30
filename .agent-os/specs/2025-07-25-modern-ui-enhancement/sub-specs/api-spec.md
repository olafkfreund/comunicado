# API Specification

This is the API specification for the spec detailed in @.agent-os/specs/2025-07-25-modern-ui-enhancement/spec.md

> Created: 2025-07-25
> Version: 1.0.0

## Plugin API Framework

### Core Plugin Trait

The foundation of the plugin system is the `Plugin` trait that all plugins must implement:

```rust
#[async_trait]
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    
    async fn initialize(&mut self, context: PluginContext) -> Result<(), PluginError>;
    async fn shutdown(&mut self) -> Result<(), PluginError>;
    
    async fn handle_event(&mut self, event: PluginEvent) -> Result<PluginResponse, PluginError>;
    fn permissions(&self) -> Vec<Permission>;
    fn config_schema(&self) -> serde_json::Value;
}
```

### Plugin Context API

The `PluginContext` provides plugins with access to core system functionality:

```rust
pub struct PluginContext {
    pub config: PluginConfig,
    pub data_store: Arc<PluginDataStore>,
    pub message_bus: Arc<MessageBus>,
    pub ui_manager: Arc<UiManager>,
}

impl PluginContext {
    // Data persistence
    pub async fn get_data(&self, key: &str) -> Result<Option<String>, PluginError>;
    pub async fn set_data(&self, key: &str, value: &str, ttl: Option<Duration>) -> Result<(), PluginError>;
    pub async fn delete_data(&self, key: &str) -> Result<(), PluginError>;
    
    // Configuration access
    pub fn get_config<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, PluginError>;
    
    // Inter-plugin communication
    pub async fn send_message(&self, target: &str, message: PluginMessage) -> Result<(), PluginError>;
    pub async fn broadcast_event(&self, event: PluginEvent) -> Result<(), PluginError>;
    
    // UI integration
    pub async fn register_ui_component(&self, component: UiComponent) -> Result<ComponentId, PluginError>;
    pub async fn update_ui_component(&self, id: ComponentId, update: UiUpdate) -> Result<(), PluginError>;
}
```

### Plugin Event System

Events are the primary mechanism for system-plugin and plugin-plugin communication:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEvent {
    // Email events
    EmailReceived { email: EmailSummary },
    EmailSent { email: EmailSummary },
    EmailDeleted { message_id: String },
    
    // Calendar events
    EventCreated { event: CalendarEvent },
    EventUpdated { event: CalendarEvent },
    EventDeleted { event_id: String },
    MeetingInvitationReceived { invitation: MeetingInvitation },
    
    // UI events
    KeyPressed { key: KeyEvent },
    UiModeChanged { mode: UiMode },
    
    // Plugin events
    PluginMessage { from: String, to: String, data: serde_json::Value },
    
    // System events
    ApplicationStartup,
    ApplicationShutdown,
    ConfigurationChanged { section: String },
}
```

### Plugin Response Types

Plugins can respond to events with various response types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginResponse {
    Handled,
    NotHandled,
    UiUpdate(UiUpdate),
    SendMessage { target: String, message: PluginMessage },
    ScheduleCallback { delay: Duration, callback: PluginCallback },
    RequestPermission(Permission),
}
```

## Calendar Integration API

### CalDAV Client Interface

```rust
#[async_trait]
pub trait CalDAVClient: Send + Sync {
    async fn authenticate(&mut self, credentials: CalDAVCredentials) -> Result<(), CalDAVError>;
    async fn get_calendars(&self) -> Result<Vec<Calendar>, CalDAVError>;
    async fn get_events(&self, calendar_id: &str, range: DateRange) -> Result<Vec<CalendarEvent>, CalDAVError>;
    
    async fn create_event(&self, calendar_id: &str, event: &CalendarEvent) -> Result<String, CalDAVError>;
    async fn update_event(&self, calendar_id: &str, event: &CalendarEvent) -> Result<(), CalDAVError>;
    async fn delete_event(&self, calendar_id: &str, event_id: &str) -> Result<(), CalDAVError>;
    
    async fn sync_calendar(&self, calendar_id: &str) -> Result<SyncResult, CalDAVError>;
}
```

### Calendar Event Management

```rust
pub struct CalendarManager {
    calendars: HashMap<String, Calendar>,
    caldav_clients: HashMap<String, Box<dyn CalDAVClient>>,
    event_store: Arc<EventStore>,
}

impl CalendarManager {
    // Event operations
    pub async fn create_event(&mut self, event: CreateEventRequest) -> Result<CalendarEvent, CalendarError>;
    pub async fn update_event(&mut self, event_id: &str, update: UpdateEventRequest) -> Result<(), CalendarError>;
    pub async fn delete_event(&mut self, event_id: &str) -> Result<(), CalendarError>;
    
    // Query operations
    pub async fn get_events_in_range(&self, range: DateRange) -> Result<Vec<CalendarEvent>, CalendarError>;
    pub async fn search_events(&self, query: &str) -> Result<Vec<CalendarEvent>, CalendarError>;
    pub async fn get_conflicts(&self, event: &CalendarEvent) -> Result<Vec<CalendarEvent>, CalendarError>;
    
    // Meeting invitation handling
    pub async fn process_meeting_invitation(&mut self, invitation: MeetingInvitation) -> Result<InvitationResponse, CalendarError>;
    pub async fn respond_to_invitation(&mut self, event_id: &str, response: AttendeeResponse) -> Result<(), CalendarError>;
}
```

## Email-Calendar Integration API

### Meeting Invitation Processor

```rust
pub struct InvitationProcessor {
    calendar_manager: Arc<CalendarManager>,
    email_client: Arc<EmailClient>,
}

impl InvitationProcessor {
    pub async fn detect_invitation(&self, email: &Email) -> Result<Option<MeetingInvitation>, ProcessorError>;
    pub async fn process_invitation(&self, invitation: MeetingInvitation) -> Result<ProcessingResult, ProcessorError>;
    pub async fn send_response(&self, response: InvitationResponse) -> Result<(), ProcessorError>;
    
    // Email to calendar conversion
    pub async fn create_event_from_email(&self, email: &Email, details: EventDetails) -> Result<CalendarEvent, ProcessorError>;
    pub async fn extract_scheduling_info(&self, email: &Email) -> Result<Option<SchedulingInfo>, ProcessorError>;
}
```

## UI Integration API

### Component Registration System

Plugins can register UI components that integrate with the main interface:

```rust
#[derive(Debug, Clone)]
pub struct UiComponent {
    pub id: String,
    pub name: String,
    pub component_type: ComponentType,
    pub position: ComponentPosition,
    pub size: ComponentSize,
    pub content: ComponentContent,
}

#[derive(Debug, Clone)]
pub enum ComponentType {
    Panel,
    StatusItem,
    MenuEntry,
    Modal,
    Notification,
}

pub struct UiManager {
    components: HashMap<ComponentId, UiComponent>,
    layout: LayoutManager,
    event_dispatcher: EventDispatcher,
}

impl UiManager {
    pub async fn register_component(&mut self, component: UiComponent) -> Result<ComponentId, UiError>;
    pub async fn update_component(&mut self, id: ComponentId, update: UiUpdate) -> Result<(), UiError>;
    pub async fn remove_component(&mut self, id: ComponentId) -> Result<(), UiError>;
    
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool, UiError>;
    pub fn render(&self, frame: &mut Frame) -> Result<(), UiError>;
}
```

## Example Plugin Implementations

### Taskwarrior Integration Plugin

```rust
pub struct TaskwarriorPlugin {
    config: TaskwarriorConfig,
    task_client: TaskwarriorClient,
    ui_component_id: Option<ComponentId>,
}

#[async_trait]
impl Plugin for TaskwarriorPlugin {
    fn name(&self) -> &str { "taskwarrior" }
    fn version(&self) -> &str { "1.0.0" }
    fn description(&self) -> &str { "Integration with Taskwarrior task management" }
    
    async fn initialize(&mut self, context: PluginContext) -> Result<(), PluginError> {
        // Register UI panel for task display
        let component = UiComponent {
            id: "taskwarrior-panel".to_string(),
            name: "Tasks".to_string(),
            component_type: ComponentType::Panel,
            position: ComponentPosition::Sidebar,
            size: ComponentSize::Medium,
            content: ComponentContent::TasksList(vec![]),
        };
        
        self.ui_component_id = Some(context.ui_manager.register_component(component).await?);
        Ok(())
    }
    
    async fn handle_event(&mut self, event: PluginEvent) -> Result<PluginResponse, PluginError> {
        match event {
            PluginEvent::EmailReceived { email } => {
                // Extract task information from email
                if let Some(task) = self.extract_task_from_email(&email).await? {
                    self.task_client.add_task(task).await?;
                    self.update_task_display().await?;
                }
                Ok(PluginResponse::Handled)
            }
            PluginEvent::KeyPressed { key } if key.code == KeyCode::Char('t') => {
                // Show task creation dialog
                Ok(PluginResponse::UiUpdate(UiUpdate::ShowModal("create-task".to_string())))
            }
            _ => Ok(PluginResponse::NotHandled)
        }
    }
    
    fn permissions(&self) -> Vec<Permission> {
        vec![
            Permission::FileSystem { path: "~/.taskrc".to_string(), access: FileAccess::Read },
            Permission::Process { command: "task".to_string() },
            Permission::UI { component_type: ComponentType::Panel },
        ]
    }
    
    fn config_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "task_data_location": {
                    "type": "string",
                    "description": "Path to Taskwarrior data directory"
                },
                "auto_create_from_email": {
                    "type": "boolean",
                    "description": "Automatically create tasks from emails"
                }
            }
        })
    }
}
```

### Chat Integration Plugin

```rust
pub struct ChatPlugin {
    config: ChatConfig,
    chat_clients: HashMap<String, Box<dyn ChatClient>>,
    notification_component: Option<ComponentId>,
}

#[async_trait]
impl Plugin for ChatPlugin {
    fn name(&self) -> &str { "chat-integration" }
    fn version(&self) -> &str { "1.0.0" }
    fn description(&self) -> &str { "Integration with various chat services" }
    
    async fn handle_event(&mut self, event: PluginEvent) -> Result<PluginResponse, PluginError> {
        match event {
            PluginEvent::PluginMessage { from, data, .. } if from == "email" => {
                // Handle email-to-chat forwarding
                if let Ok(forward_request) = serde_json::from_value::<ForwardToChatRequest>(data) {
                    self.forward_email_to_chat(forward_request).await?;
                }
                Ok(PluginResponse::Handled)
            }
            _ => Ok(PluginResponse::NotHandled)
        }
    }
    
    fn permissions(&self) -> Vec<Permission> {
        vec![
            Permission::Network { domains: vec!["slack.com".to_string(), "discord.com".to_string()] },
            Permission::UI { component_type: ComponentType::Notification },
            Permission::InterPlugin { targets: vec!["email".to_string()] },
        ]
    }
}
```

## Permission System

### Permission Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Permission {
    FileSystem { path: String, access: FileAccess },
    Network { domains: Vec<String> },
    Process { command: String },
    UI { component_type: ComponentType },
    InterPlugin { targets: Vec<String> },
    Calendar { access: CalendarAccess },
    Email { access: EmailAccess },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileAccess {
    Read,
    Write,
    ReadWrite,
}
```

### Security Sandbox

The plugin system uses WebAssembly for security isolation:

```rust
pub struct PluginSandbox {
    engine: wasmtime::Engine,
    instances: HashMap<String, PluginInstance>,
    permission_manager: Arc<PermissionManager>,
}

impl PluginSandbox {
    pub async fn load_plugin(&mut self, plugin_path: &Path) -> Result<String, SandboxError>;
    pub async fn execute_plugin(&mut self, plugin_id: &str, event: PluginEvent) -> Result<PluginResponse, SandboxError>;
    pub async fn unload_plugin(&mut self, plugin_id: &str) -> Result<(), SandboxError>;
    
    fn validate_permissions(&self, plugin_id: &str, permission: &Permission) -> Result<bool, SandboxError>;
}
```

This API specification provides a comprehensive framework for extending Comunicado through plugins while maintaining security and system integrity. The modular design allows for both simple integrations and complex multi-system workflows.