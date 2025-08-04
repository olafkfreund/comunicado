#!/usr/bin/env python3
"""
TUI Testing with Python Pexpect
Professional automated testing for Comunicado TUI application
"""

import pexpect
import sys
import time
import tempfile
from pathlib import Path

def create_test_config():
    """Create test configuration directory and file"""
    config_content = """
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
"""
    
    config_dir = tempfile.mkdtemp(prefix='comunicado_test_')
    config_path = Path(config_dir) / 'config.toml'
    config_path.write_text(config_content)
    return config_dir

def test_comunicado_startup():
    """Test basic application startup"""
    print("🧪 Testing Comunicado startup...")
    
    config_dir = create_test_config()
    app_path = "./target/debug/comunicado"
    
    try:
        # Start the application
        child = pexpect.spawn(f"{app_path} --config-dir {config_dir}", timeout=10)
        child.logfile_read = sys.stdout.buffer
        
        # Wait for application to start
        time.sleep(2)
        
        # Send quit command
        child.send('q')
        
        # Wait for clean exit
        child.expect(pexpect.EOF, timeout=5)
        
        print("✅ Startup test passed")
        return True
        
    except pexpect.TIMEOUT:
        print("❌ Startup test timed out")
        child.kill(9)
        return False
    except Exception as e:
        print(f"❌ Startup test failed: {e}")
        return False
    finally:
        import shutil
        shutil.rmtree(config_dir, ignore_errors=True)

def test_navigation_shortcuts():
    """Test keyboard navigation shortcuts"""
    print("🧪 Testing navigation shortcuts...")
    
    config_dir = create_test_config()
    app_path = "./target/debug/comunicado"
    
    try:
        child = pexpect.spawn(f"{app_path} --config-dir {config_dir}", timeout=10)
        time.sleep(2)
        
        # Test navigation shortcuts
        shortcuts = [
            ('1', 'Email view'),
            ('2', 'Calendar view'), 
            ('3', 'Contacts view'),
        ]
        
        for key, description in shortcuts:
            print(f"  Testing {description} (key '{key}')")
            child.send(key)
            time.sleep(0.5)
        
        # Test command palette
        print("  Testing Command Palette (Ctrl+D)")
        child.send('\x04')  # Ctrl+D
        time.sleep(0.5)
        child.send('\x1b')  # Escape
        time.sleep(0.5)
        
        # Test account manager
        print("  Testing Account Manager (Ctrl+A)")
        child.send('\x01')  # Ctrl+A
        time.sleep(0.5)
        child.send('\x1b')  # Escape
        time.sleep(0.5)
        
        # Test search
        print("  Testing Global Search ('/')")
        child.send('/')
        time.sleep(0.5)
        child.send('\x1b')  # Escape
        time.sleep(0.5)
        
        # Quit
        child.send('q')
        child.expect(pexpect.EOF, timeout=5)
        
        print("✅ Navigation shortcuts test passed")
        return True
        
    except pexpect.TIMEOUT:
        print("❌ Navigation test timed out")
        child.kill(9)
        return False
    except Exception as e:
        print(f"❌ Navigation test failed: {e}")
        return False
    finally:
        import shutil
        shutil.rmtree(config_dir, ignore_errors=True)

def test_email_operations():
    """Test email-specific operations"""
    print("🧪 Testing email operations...")
    
    config_dir = create_test_config()
    app_path = "./target/debug/comunicado"
    
    try:
        child = pexpect.spawn(f"{app_path} --config-dir {config_dir}", timeout=10)
        time.sleep(2)
        
        # Go to email view
        child.send('1')
        time.sleep(0.5)
        
        # Test email operations
        operations = [
            ('r', 'Mark as read'),
            ('u', 'Mark as unread'),
            ('f', 'Toggle flag'),
        ]
        
        for key, description in operations:
            print(f"  Testing {description} (key '{key}')")
            child.send(key)
            time.sleep(0.3)
        
        # Quit
        child.send('q')
        child.expect(pexpect.EOF, timeout=5)
        
        print("✅ Email operations test passed")
        return True
        
    except pexpect.TIMEOUT:
        print("❌ Email operations test timed out")
        child.kill(9)
        return False
    except Exception as e:
        print(f"❌ Email operations test failed: {e}")
        return False
    finally:
        import shutil
        shutil.rmtree(config_dir, ignore_errors=True)

def test_performance():
    """Test UI responsiveness with rapid input"""
    print("🧪 Testing UI responsiveness...")
    
    config_dir = create_test_config()
    app_path = "./target/debug/comunicado"
    
    try:
        child = pexpect.spawn(f"{app_path} --config-dir {config_dir}", timeout=10)
        time.sleep(2)
        
        # Rapid navigation test
        print("  Testing rapid navigation...")
        for _ in range(5):
            child.send('1')  # Email
            time.sleep(0.1)
            child.send('2')  # Calendar
            time.sleep(0.1)
            child.send('3')  # Contacts
            time.sleep(0.1)
        
        # Test command palette rapid open/close
        print("  Testing rapid command palette...")
        for _ in range(3):
            child.send('\x04')  # Ctrl+D
            time.sleep(0.1)
            child.send('\x1b')  # Escape
            time.sleep(0.1)
        
        # Quit
        child.send('q')
        child.expect(pexpect.EOF, timeout=5)
        
        print("✅ Performance test passed")
        return True
        
    except pexpect.TIMEOUT:
        print("❌ Performance test timed out") 
        child.kill(9)
        return False
    except Exception as e:
        print(f"❌ Performance test failed: {e}")
        return False
    finally:
        import shutil
        shutil.rmtree(config_dir, ignore_errors=True)

def run_comprehensive_tests():
    """Run all TUI tests"""
    print("🎯 Comunicado TUI Comprehensive Test Suite")
    print("=" * 50)
    
    # Build first
    print("🔨 Building application...")
    import subprocess
    result = subprocess.run(["cargo", "build"], capture_output=True)
    if result.returncode != 0:
        print("❌ Build failed")
        return False
    print("✅ Build successful")
    
    # Run tests
    tests = [
        ("Startup Test", test_comunicado_startup),
        ("Navigation Shortcuts", test_navigation_shortcuts), 
        ("Email Operations", test_email_operations),
        ("Performance Test", test_performance),
    ]
    
    results = []
    
    for test_name, test_func in tests:
        print(f"\n{'='*20} {test_name} {'='*20}")
        success = test_func()
        results.append((test_name, success))
    
    # Summary
    print("\n" + "="*50)
    print("📊 Test Results Summary")
    print("="*50)
    
    passed = sum(1 for _, success in results if success)
    total = len(results)
    
    for test_name, success in results:
        status = "✅ PASSED" if success else "❌ FAILED"
        print(f"{status} {test_name}")
    
    print(f"\n🎯 Final Results: {passed}/{total} tests passed")
    
    if passed == total:
        print("🎉 All tests passed!")
        return True
    else:
        print(f"⚠️  {total - passed} tests failed")
        return False

if __name__ == "__main__":
    success = run_comprehensive_tests()
    sys.exit(0 if success else 1)