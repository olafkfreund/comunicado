#!/usr/bin/env python3
"""
Fix tools_available field warnings in deployment modules
"""

import os
import re
from pathlib import Path

def fix_tools_available_fields(file_path):
    """Add #[allow(dead_code)] to tools_available fields."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Find and fix tools_available fields
        modified = False
        lines = content.split('\n')
        
        for i, line in enumerate(lines):
            # Look for tools_available field definitions
            if re.match(r'\s+tools_available:', line.strip()):
                # Check if already has allow annotation
                prev_line = lines[i-1] if i > 0 else ""
                if '#[allow(dead_code)]' not in prev_line:
                    # Add allow annotation
                    indent = re.match(r'(\s*)', line).group(1)
                    lines.insert(i, f'{indent}#[allow(dead_code)]')
                    modified = True
                    break  # Process one at a time
        
        if modified:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write('\n'.join(lines))
            return True
            
    except Exception as e:
        print(f"Error processing {file_path}: {e}")
        
    return False

def main():
    """Fix tools_available warnings in deployment modules."""
    print("🔧 Fixing tools_available field warnings...")
    
    deployment_files = [
        '/home/olafkfreund/Source/comunicado/src/deployment/packaging.rs',
        '/home/olafkfreund/Source/comunicado/src/deployment/distributions.rs'
    ]
    
    files_fixed = 0
    
    for file_path in deployment_files:
        if os.path.exists(file_path):
            if fix_tools_available_fields(file_path):
                print(f"✓ Fixed {file_path}")
                files_fixed += 1
        else:
            print(f"⚠️ File not found: {file_path}")
    
    print(f"\n✅ Fixed {files_fixed} files")

if __name__ == '__main__':
    main()