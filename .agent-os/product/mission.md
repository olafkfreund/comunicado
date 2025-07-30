# Product Mission

> Last Updated: 2025-07-25
> Version: 1.0.0

## Pitch

Comunicado is a modern TUI-based email and calendar client that helps terminal power users and privacy-conscious developers experience rich HTML emails, images, animations, and integrated calendar management directly in their terminal without sacrificing modern features or requiring GUI overhead.

## Users

### Primary Customers

- **Terminal Power Users**: Advanced users who prefer command-line interfaces and want rich email experience without leaving their terminal environment
- **Privacy-Conscious Users**: Individuals who prefer local email clients over web-based solutions for better privacy and data control
- **Developers and Sysadmins**: Technical professionals who live in the terminal and need efficient email management integrated into their workflow

### User Personas

**Terminal Enthusiast** (25-45 years old)
- **Role:** Software Developer/DevOps Engineer
- **Context:** Spends 8+ hours daily in terminal environments, uses vim/emacs, prefers keyboard-driven workflows
- **Pain Points:** Switching to GUI email/calendar apps breaks workflow, existing TUI clients lack modern features, poor HTML email support, fragmented calendar solutions
- **Goals:** Seamless email and calendar experience in terminal, rich content viewing, efficient keyboard navigation, integrated scheduling

**Privacy Advocate** (30-50 years old)
- **Role:** Security Engineer/Tech Lead
- **Context:** Values data privacy and local control over cloud-based solutions
- **Pain Points:** Web email clients track usage, limited control over data storage, dependency on external services
- **Goals:** Local email storage, direct IMAP control, no external dependencies for basic functionality

**System Administrator** (28-45 years old)
- **Role:** DevOps/Infrastructure Engineer
- **Context:** Manages multiple systems, needs efficient communication tools that integrate with existing terminal-based workflows
- **Pain Points:** Context switching between GUI and terminal, slow email clients, poor integration with system tools
- **Goals:** Fast email processing, integration with terminal workflows, reliable multi-account support

## The Problem

### Limited Modern TUI Email Options

Existing terminal email clients like mutt and neomutt lack modern features like native HTML rendering, image support, and intuitive user interfaces. Users are forced to choose between feature-rich GUI clients that break their terminal workflow or outdated TUI clients that can't handle modern email content.

**Our Solution:** Provide a modern TUI email client with full HTML rendering, image support, and contemporary UX design.

### Complex Setup and External Dependencies

Traditional TUI email clients require complex configuration with external tools like mbsync, offlineimap, or fetchmail, creating barriers to adoption and maintenance overhead.

**Our Solution:** Built-in IMAP and OAuth2 support with a setup wizard that eliminates external dependencies.

### Poor HTML Email Experience

Most TUI email clients either can't display HTML emails or provide such poor rendering that users must constantly switch to external browsers or GUI clients.

**Our Solution:** Terminal-native HTML rendering that supports modern terminals' capabilities including images and animations.

### Inconsistent Multi-Provider Support

Setting up multiple email accounts across different providers (Gmail, Outlook, etc.) often requires provider-specific configurations and workarounds.

**Our Solution:** Native multi-provider support with built-in OAuth2 handling for major email services.

### Fragmented Calendar and Email Integration

Most users must juggle separate applications for email and calendar management, leading to context switching and poor integration between scheduling and communication. Additionally, Linux lacks a standardized calendar solution that other applications can easily integrate with.

**Our Solution:** Integrated calendar functionality using CalDAV standards that can be shared with other Linux applications, providing a unified communication and scheduling experience.

## Differentiators

### Native Terminal HTML and Media Rendering

Unlike mutt or alpine which rely on external browsers, we provide native HTML email rendering with support for images and animations in modern terminals like foot, kitty, and wezterm. This results in seamless email viewing without context switching.

### Zero External Dependencies for Core Features

While traditional clients require mbsync, offlineimap, or similar tools, our built-in IMAP and OAuth2 implementation provides complete email functionality out of the box. This results in simpler installation, configuration, and maintenance.

### Modern Terminal UX Design

Unlike legacy TUI interfaces that feel outdated, we leverage ratatui to provide a contemporary user experience with intuitive navigation, modern keybindings, and visual polish that matches expectations from modern terminal applications.

### Integrated CalDAV Calendar Solution

While most email clients ignore calendar functionality or provide proprietary solutions, we implement standards-based CalDAV calendar support that can be shared with other Linux applications. This results in a unified calendar ecosystem where appointments and schedules are accessible across the entire desktop environment.

## Key Features

### Core Features

- **Modern TUI Interface:** Clean, intuitive interface built with ratatui for contemporary terminal experience
- **HTML Email Rendering:** Native HTML parsing and rendering optimized for terminal display
- **Image and Animation Support:** Display images and animations using modern terminal protocols (kitty, sixel, etc.)
- **Built-in IMAP Client:** Native IMAP implementation with no external dependencies required

### Authentication and Account Management

- **OAuth2 Integration:** Native OAuth2 support for Gmail, Outlook, and other major providers
- **Multi-Account Support:** Manage multiple email accounts from different providers in one interface
- **Setup Wizard:** Guided configuration process for easy account setup

### Email Management Features  

- **Maildir Support:** Compatible with standard maildir format for local email storage
- **Advanced Search:** Fast, indexed search across all emails and accounts
- **Email Filters:** Automated email organization and filtering rules
- **Folder Navigation:** Intuitive folder browsing and management interface

### Calendar and Scheduling Features

- **CalDAV Integration:** Standards-based calendar support compatible with other Linux applications
- **Meeting Invitations:** Handle calendar invites directly from email with RSVP functionality
- **Shared Calendar Access:** Multi-application calendar sharing for desktop environment integration
- **Event Management:** Create, edit, and manage calendar events with recurring event support