#!/usr/bin/env python3
"""
Test script to demonstrate mcpterm MCP server usage for TUI testing
This script shows how to use the mcpterm MCP server to test Comunicado
"""

import json
import subprocess
import sys
import time
import tempfile
from pathlib import Path

def send_mcp_request(mcpterm_path, request):
    """Send an MCP request to mcpterm and get response"""
    try:
        process = subprocess.Popen(
            [mcpterm_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        
        # Send request
        stdout, stderr = process.communicate(input=json.dumps(request) + '\n', timeout=10)
        
        if stderr:
            print(f"stderr: {stderr}")
        
        if stdout.strip():
            try:
                return json.loads(stdout.strip())
            except json.JSONDecodeError:
                print(f"Invalid JSON response: {stdout}")
                return None
                
        return None
        
    except subprocess.TimeoutExpired:
        process.kill()
        print("Request timed out")
        return None
    except Exception as e:
        print(f"Error sending MCP request: {e}")
        return None

def test_mcpterm_basic():
    """Test basic mcpterm functionality"""
    mcpterm_path = "/home/olafkfreund/.local/bin/mcpterm"
    
    print("🧪 Testing mcpterm MCP server...")
    
    # Test 1: Initialize
    print("\n1. Testing initialization...")
    init_request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "comunicado-test",
                "version": "1.0.0"
            }
        }
    }
    
    response = send_mcp_request(mcpterm_path, init_request)
    if response:
        print(f"✅ Initialize response: {response}")
    else:
        print("❌ Initialize failed")
        return False
    
    # Test 2: List tools
    print("\n2. Testing tools list...")
    tools_request = {
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    }
    
    response = send_mcp_request(mcpterm_path, tools_request)
    if response:
        print(f"✅ Tools list response: {response}")
        tools = response.get("result", {}).get("tools", [])
        print(f"Available tools: {[tool.get('name') for tool in tools]}")
    else:
        print("❌ Tools list failed")
        return False
    
    return True

def test_comunicado_with_mcpterm():
    """Test Comunicado TUI using mcpterm"""
    mcpterm_path = "/home/olafkfreund/.local/bin/mcpterm"
    app_path = "/home/olafkfreund/Source/comunicado/target/debug/comunicado"
    
    print("\n🚀 Testing Comunicado TUI with mcpterm...")
    
    # Build Comunicado first
    print("Building Comunicado...")
    build_result = subprocess.run(["cargo", "build"], 
                                cwd="/home/olafkfreund/Source/comunicado",
                                capture_output=True)
    if build_result.returncode != 0:
        print("❌ Build failed")
        return False
    print("✅ Build successful")
    
    # Create test config
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
"""
    
    with tempfile.NamedTemporaryFile(mode='w', suffix='.toml', delete=False) as f:
        f.write(config_content)
        config_path = f.name
    
    try:
        # Test using runScreen tool to start Comunicado
        print("\n📺 Testing TUI application startup...")
        
        run_request = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "runScreen",
                "arguments": {
                    "command": f"{app_path} --config {config_path}",
                    "input": "q"  # Send 'q' to quit after startup
                }
            }
        }
        
        response = send_mcp_request(mcpterm_path, run_request)
        if response:
            print("✅ TUI test response received:")
            result = response.get("result", {})
            content = result.get("content", [])
            for item in content:
                if item.get("type") == "text":
                    print(f"Screen output:\n{item.get('text', '')}")
        else:
            print("❌ TUI test failed")
            return False
            
    finally:
        # Clean up
        Path(config_path).unlink(missing_ok=True)
    
    return True

def create_tui_test_sequence():
    """Create a comprehensive TUI test sequence using mcpterm"""
    print("\n🎯 TUI Test Sequence for Comunicado")
    print("=" * 50)
    
    test_sequence = [
        {
            "name": "Application Startup",
            "command": "/home/olafkfreund/Source/comunicado/target/debug/comunicado --config /tmp/test_config.toml",
            "inputs": [],
            "wait": 2
        },
        {
            "name": "Navigate to Email (key '1')",
            "inputs": ["1"],
            "wait": 1
        },
        {
            "name": "Navigate to Calendar (key '2')",
            "inputs": ["2"],
            "wait": 1
        },
        {
            "name": "Navigate to Contacts (key '3')",
            "inputs": ["3"],
            "wait": 1
        },
        {
            "name": "Open Command Palette (Ctrl+D)",
            "inputs": ["^D"],  # Control+D
            "wait": 1
        },
        {
            "name": "Close Command Palette (Escape)",
            "inputs": ["^["],  # Escape
            "wait": 1
        },
        {
            "name": "Open Account Manager (Ctrl+A)",
            "inputs": ["^A"],  # Control+A
            "wait": 1
        },
        {
            "name": "Close Account Manager (Escape)",
            "inputs": ["^["],  # Escape
            "wait": 1
        },
        {
            "name": "Test Global Search ('/')",
            "inputs": ["/"],
            "wait": 1
        },
        {
            "name": "Close Search (Escape)",
            "inputs": ["^["],  # Escape
            "wait": 1
        },
        {
            "name": "Quit Application ('q')",
            "inputs": ["q"],
            "wait": 1
        }
    ]
    
    print("Test sequence created with the following steps:")
    for i, step in enumerate(test_sequence, 1):
        inputs_str = ", ".join(step.get("inputs", []))
        print(f"{i:2d}. {step['name']}")
        if inputs_str:
            print(f"    Inputs: {inputs_str}")
        print(f"    Wait: {step['wait']}s")
    
    print(f"\n📝 To run this sequence with mcpterm, use the 'runScreen' tool")
    print("   with the above inputs combined into a single input string.")
    
    return test_sequence

if __name__ == "__main__":
    print("🧪 Comunicado TUI Testing with mcpterm MCP Server")
    print("=" * 60)
    
    # Test 1: Basic mcpterm functionality
    if not test_mcpterm_basic():
        print("\n❌ Basic mcpterm tests failed")
        sys.exit(1)
    
    # Test 2: Test sequence planning
    create_tui_test_sequence()
    
    # Test 3: Try testing Comunicado (this might not work perfectly yet)
    print("\n" + "=" * 60)
    print("Note: The following test demonstrates the concept.")
    print("Real TUI testing requires interactive MCP session with Claude Code.")
    print("=" * 60)
    
    # Show how to use it
    print(f"""
🎯 Next Steps - How to Use mcpterm with Claude Code:

1. **MCP Server is installed**: ~/.local/bin/mcpterm
2. **Configuration created**: ~/.config/claude-code/mcp_config.json
3. **Available tools**:
   - `run`: Execute commands in stateful terminal session
   - `runScreen`: Run TUI apps and capture screen output

4. **To test Comunicado TUI**:
   Ask Claude Code to use the mcpterm tools like this:
   
   "Use the runScreen tool to start Comunicado and test keyboard shortcuts:
   - Start: ./target/debug/comunicado --config test_config.toml
   - Send keys: 1, 2, 3, ^D, ^[, ^A, ^[, /, ^[, q
   - Capture the screen output at each step"

5. **Key mappings for mcpterm**:
   - ^D = Ctrl+D (Command Palette)
   - ^A = Ctrl+A (Account Manager)  
   - ^[ = Escape
   - ^C = Ctrl+C
   - ^J = Enter
   
The mcpterm MCP server is now ready for Claude Code to use!
""")
    
    print("✅ Setup complete! mcpterm MCP server is ready for Claude Code.")