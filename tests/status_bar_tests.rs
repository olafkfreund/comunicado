use comunicado::theme::Theme;
use comunicado::ui::status_bar::{
    CalendarStatusSegment, EmailStatusSegment, NavigationHintsSegment, SeparatorStyle, StatusBar,
    StatusBarPosition, StatusSegment, SyncStatus, SystemInfoSegment,
};

#[test]
fn test_email_status_segment() {
    let segment = EmailStatusSegment {
        unread_count: 5,
        total_count: 127,
        sync_status: SyncStatus::Online,
    };

    assert_eq!(segment.content(), "Mail: 5 unread ● 127");
    assert_eq!(segment.min_width(), 20);
    assert_eq!(segment.priority(), 90);
    assert!(segment.is_visible());
}

#[test]
fn test_email_status_segment_no_unread() {
    let segment = EmailStatusSegment {
        unread_count: 0,
        total_count: 127,
        sync_status: SyncStatus::Offline,
    };

    assert_eq!(segment.content(), "Mail: ○ 127");
    assert!(segment.is_visible());
}

#[test]
fn test_email_status_segment_syncing() {
    let segment = EmailStatusSegment {
        unread_count: 3,
        total_count: 50,
        sync_status: SyncStatus::Syncing,
    };

    assert_eq!(segment.content(), "Mail: 3 unread ⟳ 50");
}

#[test]
fn test_email_status_segment_error() {
    let segment = EmailStatusSegment {
        unread_count: 0,
        total_count: 0,
        sync_status: SyncStatus::Error,
    };

    assert_eq!(segment.content(), "Mail: ⚠ 0");
}

#[test]
fn test_calendar_status_segment() {
    let segment = CalendarStatusSegment {
        next_event: Some("Team Meeting".to_string()),
        events_today: 3,
    };

    assert_eq!(segment.content(), "Cal: Next Team Meeting (3 today)");
    assert_eq!(segment.min_width(), 25);
    assert_eq!(segment.priority(), 70);
}

#[test]
fn test_calendar_status_segment_no_next_event() {
    let segment = CalendarStatusSegment {
        next_event: None,
        events_today: 2,
    };

    assert_eq!(segment.content(), "Cal: 2 events today");
}

#[test]
fn test_calendar_status_segment_no_events() {
    let segment = CalendarStatusSegment {
        next_event: None,
        events_today: 0,
    };

    assert_eq!(segment.content(), "Cal: No events");
}

#[test]
fn test_system_info_segment() {
    let segment = SystemInfoSegment {
        current_time: "14:30".to_string(),
        active_account: "work@example.com".to_string(),
    };

    assert_eq!(segment.content(), "work@example.com | 14:30");
    assert_eq!(segment.min_width(), 30);
    assert_eq!(segment.priority(), 50);
}

#[test]
fn test_navigation_hints_segment() {
    let segment = NavigationHintsSegment {
        current_pane: "Folders".to_string(),
        available_shortcuts: vec![
            ("Tab".to_string(), "Switch".to_string()),
            ("j/k".to_string(), "Navigate".to_string()),
            ("l".to_string(), "Expand".to_string()),
        ],
    };

    assert_eq!(
        segment.content(),
        "Folders | Tab: Switch | j/k: Navigate | l: Expand"
    );
    assert_eq!(segment.min_width(), 40);
    assert_eq!(segment.priority(), 30);
}

#[test]
fn test_navigation_hints_segment_many_shortcuts() {
    let segment = NavigationHintsSegment {
        current_pane: "Messages".to_string(),
        available_shortcuts: vec![
            ("Tab".to_string(), "Switch".to_string()),
            ("j/k".to_string(), "Navigate".to_string()),
            ("Enter".to_string(), "Open".to_string()),
            ("d".to_string(), "Delete".to_string()), // This should be truncated
            ("r".to_string(), "Reply".to_string()),  // This should be truncated
        ],
    };

    // Should only show first 3 shortcuts
    assert_eq!(
        segment.content(),
        "Messages | Tab: Switch | j/k: Navigate | Enter: Open"
    );
}

#[test]
fn test_status_bar_creation() {
    let status_bar = StatusBar::new(StatusBarPosition::Bottom);
    assert_eq!(
        status_bar.get_status_summary(),
        "StatusBar: 0 segments, position: Bottom, style: Powerline"
    );
}

#[test]
fn test_status_bar_add_segments() {
    let mut status_bar = StatusBar::new(StatusBarPosition::Top);

    let email_segment = EmailStatusSegment {
        unread_count: 5,
        total_count: 100,
        sync_status: SyncStatus::Online,
    };

    let system_segment = SystemInfoSegment {
        current_time: "10:30".to_string(),
        active_account: "test@example.com".to_string(),
    };

    status_bar.add_segment("email".to_string(), email_segment);
    status_bar.add_segment("system".to_string(), system_segment);

    assert_eq!(
        status_bar.get_status_summary(),
        "StatusBar: 2 segments, position: Top, style: Powerline"
    );
}

#[test]
fn test_status_bar_remove_segment() {
    let mut status_bar = StatusBar::new(StatusBarPosition::Bottom);

    let email_segment = EmailStatusSegment {
        unread_count: 0,
        total_count: 50,
        sync_status: SyncStatus::Online,
    };

    status_bar.add_segment("email".to_string(), email_segment);
    assert_eq!(
        status_bar.get_status_summary(),
        "StatusBar: 1 segments, position: Bottom, style: Powerline"
    );

    status_bar.remove_segment("email");
    assert_eq!(
        status_bar.get_status_summary(),
        "StatusBar: 0 segments, position: Bottom, style: Powerline"
    );
}

#[test]
fn test_status_bar_segment_ordering() {
    let mut status_bar = StatusBar::new(StatusBarPosition::Bottom);

    // Add segments in reverse priority order
    let system_segment = SystemInfoSegment {
        current_time: "12:00".to_string(),
        active_account: "test@example.com".to_string(),
    }; // Priority 50

    let email_segment = EmailStatusSegment {
        unread_count: 1,
        total_count: 10,
        sync_status: SyncStatus::Online,
    }; // Priority 90

    let nav_segment = NavigationHintsSegment {
        current_pane: "Test".to_string(),
        available_shortcuts: vec![],
    }; // Priority 30

    status_bar.add_segment("system".to_string(), system_segment);
    status_bar.add_segment("email".to_string(), email_segment);
    status_bar.add_segment("navigation".to_string(), nav_segment);

    // Should be ordered by priority: email (90), system (50), navigation (30)
    let order = vec![
        "email".to_string(),
        "system".to_string(),
        "navigation".to_string(),
    ];
    status_bar.set_segment_order(order);

    assert_eq!(
        status_bar.get_status_summary(),
        "StatusBar: 3 segments, position: Bottom, style: Powerline"
    );
}

#[test]
fn test_status_bar_separator_styles() {
    let mut status_bar = StatusBar::new(StatusBarPosition::Top);

    status_bar.set_separator_style(SeparatorStyle::Simple);
    assert_eq!(
        status_bar.get_status_summary(),
        "StatusBar: 0 segments, position: Top, style: Simple"
    );

    status_bar.set_separator_style(SeparatorStyle::Minimal);
    assert_eq!(
        status_bar.get_status_summary(),
        "StatusBar: 0 segments, position: Top, style: Minimal"
    );

    status_bar.set_separator_style(SeparatorStyle::Powerline);
    assert_eq!(
        status_bar.get_status_summary(),
        "StatusBar: 0 segments, position: Top, style: Powerline"
    );
}

#[test]
fn test_sync_status_values() {
    assert_eq!(SyncStatus::Online, SyncStatus::Online);
    assert_eq!(SyncStatus::Syncing, SyncStatus::Syncing);
    assert_eq!(SyncStatus::Offline, SyncStatus::Offline);
    assert_eq!(SyncStatus::Error, SyncStatus::Error);

    assert_ne!(SyncStatus::Online, SyncStatus::Offline);
    assert_ne!(SyncStatus::Syncing, SyncStatus::Error);
}

#[test]
fn test_status_segment_styling() {
    let theme = Theme::gruvbox_dark();

    // Test email segment with unread messages (should have custom style)
    let email_segment_unread = EmailStatusSegment {
        unread_count: 5,
        total_count: 100,
        sync_status: SyncStatus::Online,
    };

    assert!(email_segment_unread.custom_style(&theme).is_some());

    // Test email segment without unread messages (should not have custom style)
    let email_segment_read = EmailStatusSegment {
        unread_count: 0,
        total_count: 100,
        sync_status: SyncStatus::Online,
    };

    assert!(email_segment_read.custom_style(&theme).is_none());

    // Test navigation segment (should have custom style)
    let nav_segment = NavigationHintsSegment {
        current_pane: "Test".to_string(),
        available_shortcuts: vec![],
    };

    assert!(nav_segment.custom_style(&theme).is_some());
}

#[test]
fn test_status_bar_default() {
    let status_bar = StatusBar::default();
    assert_eq!(
        status_bar.get_status_summary(),
        "StatusBar: 0 segments, position: Bottom, style: Powerline"
    );
}
