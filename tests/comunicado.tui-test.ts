/**
 * Microsoft TUI-Test Configuration for Comunicado
 * Comprehensive automated TUI testing
 */

import { test, expect } from "@microsoft/tui-test";
import { writeFileSync, unlinkSync, mkdirSync, rmSync } from "fs";
import { tmpdir } from "os";
import { join } from "path";

// Test configuration helper
function createTestConfig(): string {
  const configContent = `
[ui]
theme = "dark"
enable_animations = false

[email]
database_path = "/tmp/comunicado_test_email.db"

[calendar]
database_path = "/tmp/comunicado_test_calendar.db"

[notification]
enable_desktop_notifications = false

[keyboard]
enable_custom_shortcuts = true
`;

  const configDir = join(tmpdir(), `comunicado_test_${Date.now()}`);
  mkdirSync(configDir, { recursive: true });
  const configPath = join(configDir, 'config.toml');
  writeFileSync(configPath, configContent);
  return configDir;
}

// Cleanup helper
function cleanup(configDir: string) {
  try {
    rmSync(configDir, { recursive: true, force: true });
  } catch (e) {
    // Ignore cleanup errors
  }
}

test.describe("Comunicado TUI Application", () => {
  
  test("should start and show main interface", async ({ terminal }) => {
    const configPath = createTestConfig();
    
    try {
      await terminal.sendText(`./target/debug/comunicado --config-dir ${configPath}`);
      await terminal.waitForText("Comunicado", { timeout: 5000 });
      
      // Quit the application
      await terminal.sendKeys("q");
      
    } finally {
      cleanup(configPath);
    }
  });

  test("should navigate between views with keyboard shortcuts", async ({ terminal }) => {
    const configPath = createTestConfig();
    
    try {
      await terminal.sendText(`./target/debug/comunicado --config-dir ${configPath}`);
      await terminal.waitForText("Comunicado", { timeout: 5000 });
      
      // Test Email view (1)
      await terminal.sendKeys("1");
      await expect(terminal.getByText("Email")).toBeVisible();
      
      // Test Calendar view (2)
      await terminal.sendKeys("2");
      await expect(terminal.getByText("Calendar")).toBeVisible();
      
      // Test Contacts view (3)
      await terminal.sendKeys("3");
      await expect(terminal.getByText("Contacts")).toBeVisible();
      
      // Return to Email view
      await terminal.sendKeys("1");
      
      // Quit
      await terminal.sendKeys("q");
      
    } finally {
      cleanup(configPath);
    }
  });

  test("should open and close command palette", async ({ terminal }) => {
    const configPath = createTestConfig();
    
    try {
      await terminal.sendText(`./target/debug/comunicado --config-dir ${configPath}`);
      await terminal.waitForText("Comunicado", { timeout: 5000 });
      
      // Open command palette with Ctrl+D
      await terminal.sendKeys("ctrl+d");
      await expect(terminal.getByText("Command")).toBeVisible();
      
      // Close with Escape
      await terminal.sendKeys("Escape");
      
      // Quit
      await terminal.sendKeys("q");
      
    } finally {
      cleanup(configPath);
    }
  });

  test("should open and close account manager", async ({ terminal }) => {
    const configPath = createTestConfig();
    
    try {
      await terminal.sendText(`./target/debug/comunicado --config-dir ${configPath}`);
      await terminal.waitForText("Comunicado", { timeout: 5000 });
      
      // Open account manager with Ctrl+A
      await terminal.sendKeys("ctrl+a");
      await expect(terminal.getByText("Account")).toBeVisible();
      
      // Close with Escape
      await terminal.sendKeys("Escape");
      
      // Quit
      await terminal.sendKeys("q");
      
    } finally {
      cleanup(configPath);
    }
  });

  test("should handle global search", async ({ terminal }) => {
    const configPath = createTestConfig();
    
    try {
      await terminal.sendText(`./target/debug/comunicado --config-dir ${configPath}`);
      await terminal.waitForText("Comunicado", { timeout: 5000 });
      
      // Open search with '/'
      await terminal.sendKeys("/");
      await expect(terminal.getByText("Search")).toBeVisible();
      
      // Close with Escape
      await terminal.sendKeys("Escape");
      
      // Quit
      await terminal.sendKeys("q");
      
    } finally {
      cleanup(configPath);
    }
  });

  test("should handle email operations", async ({ terminal }) => {
    const configPath = createTestConfig();
    
    try {
      await terminal.sendText(`./target/debug/comunicado --config-dir ${configPath}`);
      await terminal.waitForText("Comunicado", { timeout: 5000 });
      
      // Go to email view
      await terminal.sendKeys("1");
      await expect(terminal.getByText("Email")).toBeVisible();
      
      // Test email operations (these might not have visible feedback)
      await terminal.sendKeys("r"); // Mark as read
      await terminal.sendKeys("u"); // Mark as unread
      await terminal.sendKeys("f"); // Toggle flag
      
      // Quit
      await terminal.sendKeys("q");
      
    } finally {
      cleanup(configPath);
    }
  });

  test("should handle help system", async ({ terminal }) => {
    const configPath = createTestConfig();
    
    try {
      await terminal.sendText(`./target/debug/comunicado --config-dir ${configPath}`);
      await terminal.waitForText("Comunicado", { timeout: 5000 });
      
      // Open help with F1
      await terminal.sendKeys("F1");
      await expect(terminal.getByText("Help")).toBeVisible();
      
      // Close with Escape
      await terminal.sendKeys("Escape");
      
      // Quit
      await terminal.sendKeys("q");
      
    } finally {
      cleanup(configPath);
    }
  });

  test("should handle rapid navigation without crashes", async ({ terminal }) => {
    const configPath = createTestConfig();
    
    try {
      await terminal.sendText(`./target/debug/comunicado --config-dir ${configPath}`);
      await terminal.waitForText("Comunicado", { timeout: 5000 });
      
      // Rapid navigation test
      for (let i = 0; i < 5; i++) {
        await terminal.sendKeys("1"); // Email
        await terminal.sendKeys("2"); // Calendar  
        await terminal.sendKeys("3"); // Contacts
      }
      
      // Rapid command palette test
      for (let i = 0; i < 3; i++) {
        await terminal.sendKeys("ctrl+d");
        await terminal.sendKeys("Escape");
      }
      
      // Should still be responsive
      await terminal.sendKeys("1");
      await expect(terminal.getByText("Email")).toBeVisible();
      
      // Quit
      await terminal.sendKeys("q");
      
    } finally {
      cleanup(configPath);
    }
  });

  test("should handle calendar view switching", async ({ terminal }) => {
    const configPath = createTestConfig();
    
    try {
      await terminal.sendText(`./target/debug/comunicado --config-dir ${configPath}`);
      await terminal.waitForText("Comunicado", { timeout: 5000 });
      
      // Go to calendar view
      await terminal.sendKeys("2");
      await expect(terminal.getByText("Calendar")).toBeVisible();
      
      // Test calendar view shortcuts
      await terminal.sendKeys("d"); // Day view
      await terminal.sendKeys("w"); // Week view
      await terminal.sendKeys("m"); // Month view
      
      // Quit
      await terminal.sendKeys("q");
      
    } finally {
      cleanup(configPath);
    }
  });

  test("comprehensive keyboard shortcut validation", async ({ terminal }) => {
    const configPath = createTestConfig();
    
    try {
      await terminal.sendText(`./target/debug/comunicado --config-dir ${configPath}`);
      await terminal.waitForText("Comunicado", { timeout: 5000 });
      
      // Test all major shortcuts in sequence
      const shortcuts = [
        { key: "1", description: "Email view" },
        { key: "2", description: "Calendar view" }, 
        { key: "3", description: "Contacts view" },
        { key: "ctrl+d", description: "Command palette", close: "Escape" },
        { key: "ctrl+a", description: "Account manager", close: "Escape" },
        { key: "/", description: "Search", close: "Escape" },
        { key: "F1", description: "Help", close: "Escape" },
      ];
      
      for (const shortcut of shortcuts) {
        console.log(`Testing ${shortcut.description} (${shortcut.key})`);
        await terminal.sendKeys(shortcut.key);
        
        if (shortcut.close) {
          await terminal.sendKeys(shortcut.close);
        }
        
        // Small delay between shortcuts
        await new Promise(resolve => setTimeout(resolve, 200));
      }
      
      // Final test - should still be responsive
      await terminal.sendKeys("1");
      
      // Quit
      await terminal.sendKeys("q");
      
    } finally {
      cleanup(configPath);
    }
  });

});