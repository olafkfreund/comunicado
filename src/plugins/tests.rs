//! Comprehensive tests for the plugin architecture
//!
//! These tests validate all aspects of the plugin system including:
//! - Plugin loading and registration
//! - Plugin lifecycle management
//! - Plugin communication and functionality
//! - Error handling and recovery
//! - Performance and reliability

use super::*;
use crate::email::StoredMessage;
use crate::calendar::event::Event;
use crate::plugins::core::PluginConfig;
use crate::plugins::types::{
    UIPosition, UICapability, UIInputResult, CalendarCapability, 
    NotificationMessage, NotificationType
};

use tokio_test;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

/// Test plugin manager initialization and basic operations
#[tokio::test]
async fn test_plugin_manager_initialization() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dirs = vec![temp_dir.path().to_path_buf()];
    let app_version = "1.0.0".to_string();
    let base_data_dir = temp_dir.path().join("data");

    let mut manager = PluginManager::new(
        plugin_dirs,
        app_version,
        base_data_dir,
    ).unwrap();

    // Test initialization
    assert!(manager.initialize().await.is_ok());

    // Test initial state
    assert_eq!(manager.get_loaded_plugins().len(), 0);
    assert!(!manager.is_plugin_loaded(uuid::Uuid::new_v4()));
}

/// Test plugin registration and unregistration
#[tokio::test]
async fn test_plugin_registry_operations() {
    let mut registry = PluginRegistry::new();

    // Create example plugin
    let plugin = Box::new(examples::ExampleEmailPlugin::new());
    let plugin_info = plugin.info();
    let plugin_id = plugin_info.id;

    // Test registration
    assert!(registry.register_plugin(plugin, plugin_info.clone()).is_ok());
    assert!(registry.is_loaded(&plugin_id));
    assert_eq!(registry.plugin_count(), 1);

    // Test retrieval
    assert!(registry.get_plugin(&plugin_id).is_some());
    assert!(registry.get_plugin_info(&plugin_id).is_some());

    // Test status management
    registry.set_plugin_status(plugin_id, PluginStatus::Running);
    assert_eq!(registry.get_plugin_status(&plugin_id), Some(PluginStatus::Running));

    // Test unregistration
    assert!(registry.unregister_plugin(&plugin_id).is_ok());
    assert!(!registry.is_loaded(&plugin_id));
    assert_eq!(registry.plugin_count(), 0);
}

/// Test plugin by type categorization
#[tokio::test]
async fn test_plugin_type_categorization() {
    let mut registry = PluginRegistry::new();

    // Register plugins of different types
    let email_plugin = Box::new(examples::ExampleEmailPlugin::new());
    let email_info = email_plugin.info();
    registry.register_plugin(email_plugin, email_info).unwrap();

    let ui_plugin = Box::new(examples::ExampleUIPlugin::new());
    let ui_info = ui_plugin.info();
    registry.register_plugin(ui_plugin, ui_info).unwrap();

    let calendar_plugin = Box::new(examples::ExampleCalendarPlugin::new());
    let calendar_info = calendar_plugin.info();
    registry.register_plugin(calendar_plugin, calendar_info).unwrap();

    // Test type-based queries
    assert_eq!(registry.plugin_count_by_type(PluginType::Email), 1);
    assert_eq!(registry.plugin_count_by_type(PluginType::UI), 1);
    assert_eq!(registry.plugin_count_by_type(PluginType::Calendar), 1);
    assert_eq!(registry.plugin_count_by_type(PluginType::Notification), 0);

    let email_plugins = registry.get_plugins_by_type(PluginType::Email);
    assert_eq!(email_plugins.len(), 1);
    assert_eq!(email_plugins[0].plugin_type, PluginType::Email);
}

/// Test plugin configuration management
#[tokio::test]
async fn test_plugin_configuration() {
    let plugin_id = uuid::Uuid::new_v4();
    let mut config = PluginConfig::new(plugin_id);

    // Test setting configuration values
    assert!(config.set_config("test_key", "test_value").is_ok());
    assert!(config.set_config("number_key", 42).is_ok());

    // Test getting configuration values
    let string_value: String = config.get_config("test_key").unwrap();
    assert_eq!(string_value, "test_value");

    let number_value: i32 = config.get_config("number_key").unwrap();
    assert_eq!(number_value, 42);

    // Test missing key
    let result: Result<String, _> = config.get_config("missing_key");
    assert!(result.is_err());
}

/// Test email plugin functionality
#[tokio::test]
async fn test_email_plugin_functionality() {
    let mut email_plugin = examples::ExampleEmailPlugin::new();
    let config = PluginConfig::new(email_plugin.info().id);

    // Initialize plugin
    assert!(email_plugin.initialize(&config).is_ok());
    assert!(email_plugin.start().is_ok());

    // Create test email message
    let message = create_test_email("Test spam message with free offer", "Regular email body");

    // Test email processing
    let context = EmailPluginContext {
        account_id: "test_account".to_string(),
        folder_name: "INBOX".to_string(),
        context_data: std::collections::HashMap::new(),
    };

    let result = email_plugin.process_incoming_email(&message, &context).await;
    assert!(result.is_ok());

    // The example plugin should detect spam and move it
    match result.unwrap() {
        EmailProcessResult::Move(folder) => {
            assert_eq!(folder, "Spam");
        }
        _ => panic!("Expected spam email to be moved to Spam folder"),
    }

    // Test filtering
    let messages = vec![
        create_test_email("Normal email", "Regular content"),
        create_test_email("SPAM: Free offer!", "Click here for free stuff"),
    ];

    let filter_results = email_plugin.filter_emails(&messages, &context).await.unwrap();
    assert_eq!(filter_results.len(), 2);
    assert_eq!(filter_results[0], true);  // Normal email passes filter
    assert_eq!(filter_results[1], false); // Spam email is filtered out

    assert!(email_plugin.stop().is_ok());
}

/// Test UI plugin functionality
#[tokio::test]
async fn test_ui_plugin_functionality() {
    let mut ui_plugin = examples::ExampleUIPlugin::new();
    let config = PluginConfig::new(ui_plugin.info().id);

    // Initialize plugin
    assert!(ui_plugin.initialize(&config).is_ok());
    assert!(ui_plugin.start().is_ok());

    // Test layout preferences
    let preferences = ui_plugin.get_layout_preferences();
    assert!(matches!(preferences.preferred_position, UIPosition::Bottom));
    assert_eq!(preferences.min_size, (20, 3));

    // Test capabilities
    let capabilities = ui_plugin.get_ui_capabilities();
    assert!(capabilities.contains(&UICapability::CustomWidgets));
    assert!(capabilities.contains(&UICapability::KeyboardShortcuts));

    // Test input handling
    let context = UIPluginContext {
        app_state: "main".to_string(),
        selected_data: None,
        theme_data: std::collections::HashMap::new(),
        screen_size: (80, 24),
    };

    let key_event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::F(5),
        crossterm::event::KeyModifiers::NONE,
    );

    let input_result = ui_plugin.handle_input(key_event, &context).await.unwrap();
    match input_result {
        UIInputResult::Action(action, _) => {
            assert_eq!(action, "refresh_plugin");
        }
        _ => panic!("Expected F5 key to trigger refresh action"),
    }

    assert!(ui_plugin.stop().is_ok());
}

/// Test calendar plugin functionality
#[tokio::test]
async fn test_calendar_plugin_functionality() {
    let mut calendar_plugin = examples::ExampleCalendarPlugin::new();
    let config = PluginConfig::new(calendar_plugin.info().id);

    // Initialize plugin
    assert!(calendar_plugin.initialize(&config).is_ok());
    assert!(calendar_plugin.start().is_ok());

    // Create test event
    let event = create_test_event("Test Meeting", "Important business meeting");

    // Test event processing
    let context = CalendarPluginContext {
        calendar_id: "test_calendar".to_string(),
        timezone: "UTC".to_string(),
        context_data: std::collections::HashMap::new(),
    };

    let result = calendar_plugin.process_event(&event, &context).await;
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), CalendarEventResult::Success));

    // Test calendar sources
    let sources = calendar_plugin.get_calendar_sources().await.unwrap();
    assert!(sources.is_empty()); // Example plugin returns empty list

    // Test capabilities
    let capabilities = calendar_plugin.get_calendar_capabilities();
    assert!(capabilities.contains(&CalendarCapability::EventCreation));
    assert!(capabilities.contains(&CalendarCapability::EventModification));

    assert!(calendar_plugin.stop().is_ok());
}

/// Test notification plugin functionality
#[tokio::test]
async fn test_notification_plugin_functionality() {
    let mut notification_plugin = examples::ExampleNotificationPlugin::new();
    let config = PluginConfig::new(notification_plugin.info().id);

    // Initialize plugin
    assert!(notification_plugin.initialize(&config).is_ok());
    assert!(notification_plugin.start().is_ok());

    // Create test notification
    let notification = NotificationMessage {
        title: "Test Notification".to_string(),
        body: "This is a test notification".to_string(),
        notification_type: NotificationType::NewEmail,
        priority: NotificationPriority::Normal,
        actions: vec![],
        data: std::collections::HashMap::new(),
    };

    // Test notification sending
    let context = NotificationPluginContext {
        user_preferences: std::collections::HashMap::new(),
        channel: "desktop".to_string(),
        context_data: std::collections::HashMap::new(),
    };

    let result = notification_plugin.send_notification(&notification, &context).await;
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), NotificationResult::Sent));

    // Test supported types
    let supported_types = notification_plugin.get_supported_types();
    assert!(supported_types.contains(&NotificationType::NewEmail));
    assert!(supported_types.contains(&NotificationType::CalendarReminder));

    // Test capabilities
    let capabilities = notification_plugin.get_notification_capabilities();
    assert!(capabilities.contains(&NotificationCapability::Desktop));
    assert!(capabilities.contains(&NotificationCapability::Sound));

    assert!(notification_plugin.stop().is_ok());
}

/// Test search plugin functionality
#[tokio::test]
async fn test_search_plugin_functionality() {
    let mut search_plugin = examples::ExampleSearchPlugin::new();
    let config = PluginConfig::new(search_plugin.info().id);

    // Initialize plugin
    assert!(search_plugin.initialize(&config).is_ok());
    assert!(search_plugin.start().is_ok());

    // Create test search query
    let query = SearchQuery {
        query: "test search".to_string(),
        filters: std::collections::HashMap::new(),
        sort_by: None,
        limit: Some(10),
        offset: None,
    };

    // Test search functionality
    let context = SearchPluginContext {
        scope: SearchScope::All,
        preferences: std::collections::HashMap::new(),
        context_data: std::collections::HashMap::new(),
    };

    let result = search_plugin.search(&query, &context).await;
    assert!(result.is_ok());

    let search_result = result.unwrap();
    assert_eq!(search_result.total_count, 1);
    assert_eq!(search_result.items.len(), 1);
    assert!(search_result.execution_time.as_millis() > 0);

    // Test suggestions
    let suggestions = search_plugin.get_suggestions("test", &context).await.unwrap();
    assert!(suggestions.len() > 0);
    assert!(suggestions.iter().any(|s| s.contains("test")));

    // Test capabilities
    let capabilities = search_plugin.get_search_capabilities();
    assert!(capabilities.contains(&SearchCapability::FullText));
    assert!(capabilities.contains(&SearchCapability::Fuzzy));

    assert!(search_plugin.stop().is_ok());
}

/// Test plugin loading strategies
#[tokio::test]
async fn test_plugin_loading_strategies() {
    let mut loader = PluginLoader::new();

    // Test loading built-in example plugins
    let email_plugin_info = examples::ExampleEmailPlugin::new().info();
    let result = loader.load_plugin(&email_plugin_info).await;
    assert!(result.is_ok());

    let ui_plugin_info = examples::ExampleUIPlugin::new().info();
    let result = loader.load_plugin(&ui_plugin_info).await;
    assert!(result.is_ok());

    // Test loading non-existent plugin
    let fake_info = PluginInfo::new(
        "Fake Plugin".to_string(),
        "1.0.0".to_string(),
        "Non-existent plugin".to_string(),
        "Test".to_string(),
        PluginType::Utility,
        "1.0.0".to_string(),
    );

    let result = loader.load_plugin(&fake_info).await;
    assert!(result.is_err());
}

/// Test plugin dependency validation
#[tokio::test]
async fn test_plugin_dependency_validation() {
    let registry = PluginRegistry::new();

    // Create plugin with dependencies
    let mut plugin_info = examples::ExampleEmailPlugin::new().info();
    plugin_info.dependencies = vec![
        PluginDependency {
            name: "required_plugin".to_string(),
            version: "1.0.0".to_string(),
            optional: false,
        },
        PluginDependency {
            name: "optional_plugin".to_string(),
            version: "1.0.0".to_string(),
            optional: true,
        },
    ];

    // Test validation with missing dependencies
    let missing_deps = registry.validate_dependencies(&plugin_info);
    assert_eq!(missing_deps.len(), 1);
    assert_eq!(missing_deps[0], "required_plugin");
}

/// Test plugin error handling and recovery
#[tokio::test]
async fn test_plugin_error_handling() {
    let mut registry = PluginRegistry::new();

    // Test duplicate registration
    let plugin1 = Box::new(examples::ExampleEmailPlugin::new());
    let plugin_info = plugin1.info();
    let plugin_id = plugin_info.id;

    assert!(registry.register_plugin(plugin1, plugin_info.clone()).is_ok());

    let plugin2 = Box::new(examples::ExampleEmailPlugin::new());
    let result = registry.register_plugin(plugin2, plugin_info);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PluginError::AlreadyLoaded(_)));

    // Test unregistering non-existent plugin
    let fake_id = uuid::Uuid::new_v4();
    let result = registry.unregister_plugin(&fake_id);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PluginError::NotFound(_)));
}

/// Test plugin performance and metrics
#[tokio::test]
async fn test_plugin_performance() {
    let mut email_plugin = examples::ExampleEmailPlugin::new();
    let config = PluginConfig::new(email_plugin.info().id);

    email_plugin.initialize(&config).unwrap();
    email_plugin.start().unwrap();

    let context = EmailPluginContext {
        account_id: "test_account".to_string(),
        folder_name: "INBOX".to_string(),
        context_data: std::collections::HashMap::new(),
    };

    // Test processing multiple emails
    let start_time = std::time::Instant::now();
    
    for i in 0..100 {
        let message = create_test_email(
            &format!("Email {}", i),
            "Normal email content"
        );
        
        let result = email_plugin.process_incoming_email(&message, &context).await;
        assert!(result.is_ok());
    }

    let duration = start_time.elapsed();
    
    // Performance assertion: should process 100 emails in under 1 second
    assert!(duration.as_secs() < 1, "Email processing took too long: {:?}", duration);

    email_plugin.stop().unwrap();
}

/// Test registry statistics and reporting
#[tokio::test]
async fn test_registry_statistics() {
    let mut registry = PluginRegistry::new();

    // Register plugins of different types
    let email_plugin = Box::new(examples::ExampleEmailPlugin::new());
    let email_info = email_plugin.info();
    registry.register_plugin(email_plugin, email_info).unwrap();

    let ui_plugin = Box::new(examples::ExampleUIPlugin::new());
    let ui_info = ui_plugin.info();
    registry.register_plugin(ui_plugin, ui_info).unwrap();

    let notification_plugin = Box::new(examples::ExampleNotificationPlugin::new());
    let notification_info = notification_plugin.info();
    registry.register_plugin(notification_plugin, notification_info).unwrap();

    // Test statistics
    let stats = registry.get_statistics();
    assert_eq!(stats.total_plugins, 3);
    assert_eq!(stats.email_plugins, 1);
    assert_eq!(stats.ui_plugins, 1);
    assert_eq!(stats.notification_plugins, 1);
    assert_eq!(stats.running_plugins, 0); // None are running yet
    assert!(stats.is_healthy()); // No error plugins

    // Test distribution
    let distribution = stats.get_type_distribution();
    let email_count = distribution.iter()
        .find(|(t, _)| *t == PluginType::Email)
        .map(|(_, count)| *count)
        .unwrap_or(0);
    assert_eq!(email_count, 1);
}

// ============================================================================
// Helper Functions for Tests
// ============================================================================

/// Create a test email message
fn create_test_email(subject: &str, body: &str) -> StoredMessage {
    StoredMessage {
        id: uuid::Uuid::new_v4(),
        account_id: "test_account".to_string(),
        folder_name: "INBOX".to_string(),
        imap_uid: 1,
        message_id: Some("test@example.com".to_string()),
        thread_id: None,
        in_reply_to: None,
        references: vec![],
        subject: subject.to_string(),
        from_addr: "sender@example.com".to_string(),
        from_name: Some("Test Sender".to_string()),
        to_addrs: vec!["recipient@example.com".to_string()],
        cc_addrs: vec![],
        bcc_addrs: vec![],
        reply_to: None,
        date: chrono::Utc::now(),
        body_text: Some(body.to_string()),
        body_html: None,
        attachments: vec![],
        flags: vec![],
        labels: vec![],
        size: Some(body.len() as u64),
        priority: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: chrono::Utc::now(),
        sync_version: 1,
        is_draft: false,
        is_deleted: false,
    }
}

/// Create a test calendar event
fn create_test_event(title: &str, description: &str) -> Event {
    Event {
        id: uuid::Uuid::new_v4().to_string(),
        calendar_id: "test_calendar".to_string(),
        title: title.to_string(),
        description: Some(description.to_string()),
        start_time: chrono::Utc::now(),
        end_time: chrono::Utc::now() + chrono::Duration::hours(1),
        all_day: false,
        location: None,
        attendees: vec![],
        recurrence_rule: None,
        reminder: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_synced: Some(chrono::Utc::now()),
    }
}