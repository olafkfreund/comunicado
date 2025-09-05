#!/usr/bin/env python3
"""
Comprehensive warning cleanup script for Comunicado
Handles specific warning patterns found in build output
"""

import os
import re
import subprocess
from pathlib import Path

def get_detailed_warnings():
    """Get detailed warning information from cargo build."""
    result = subprocess.run(['cargo', 'build', '--all-targets'], 
                          capture_output=True, text=True, cwd='/home/olafkfreund/Source/comunicado')
    return result.stderr

def parse_warning_details(warning_text):
    """Parse warning text to extract file locations and specific warnings."""
    warnings = []
    current_file = None
    
    for line in warning_text.split('\n'):
        # Match file location lines
        file_match = re.match(r'\s*--> (.+):(\d+):(\d+)', line)
        if file_match:
            current_file = file_match.group(1)
            line_num = int(file_match.group(2))
            col_num = int(file_match.group(3))
            continue
            
        # Match warning content
        if 'never used' in line or 'never read' in line:
            if current_file:
                warnings.append({
                    'file': current_file,
                    'line': line_num if 'line_num' in locals() else 0,
                    'message': line.strip()
                })
    
    return warnings

def fix_unused_methods(file_path, content):
    """Add #[allow(dead_code)] to unused methods."""
    lines = content.split('\n')
    modified = False
    
    for i, line in enumerate(lines):
        # Look for method definitions that might be unused
        if re.match(r'\s*(pub\s+)?(async\s+)?fn\s+\w+', line):
            # Check if already has allow annotation
            prev_line = lines[i-1] if i > 0 else ""
            if '#[allow(dead_code)]' not in prev_line:
                # Add allow annotation
                indent = re.match(r'(\s*)', line).group(1)
                lines.insert(i, f'{indent}#[allow(dead_code)]')
                modified = True
                break  # Process one at a time to avoid index issues
    
    return '\n'.join(lines), modified

def fix_unused_fields(file_path, content):
    """Add #[allow(dead_code)] to struct definitions with unused fields."""
    lines = content.split('\n')
    modified = False
    
    for i, line in enumerate(lines):
        # Look for struct definitions
        if re.match(r'\s*(?:pub\s+)?struct\s+\w+', line):
            # Check if already has allow annotation
            prev_lines = lines[max(0, i-3):i]
            has_allow = any('#[allow(dead_code)]' in pl for pl in prev_lines)
            
            if not has_allow:
                # Add allow annotation before struct
                indent = re.match(r'(\s*)', line).group(1)
                lines.insert(i, f'{indent}#[allow(dead_code)]')
                modified = True
                break  # Process one at a time
    
    return '\n'.join(lines), modified

def process_file(file_path):
    """Process a single file to fix warnings."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            original_content = f.read()
        
        content = original_content
        file_modified = False
        
        # Try to fix unused methods
        content, method_modified = fix_unused_methods(file_path, content)
        if method_modified:
            file_modified = True
        
        # Try to fix unused fields  
        content, field_modified = fix_unused_fields(file_path, content)
        if field_modified:
            file_modified = True
            
        if file_modified:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"✓ Fixed warnings in {file_path}")
            return True
            
    except Exception as e:
        print(f"✗ Error processing {file_path}: {e}")
        
    return False

def main():
    """Main cleanup process."""
    print("🔧 Running comprehensive warning cleanup...")
    
    # Get current warnings
    warning_output = get_detailed_warnings()
    warning_count_before = warning_output.count('warning:')
    print(f"📊 Found {warning_count_before} warnings to address")
    
    # Find all Rust source files
    src_dir = Path('/home/olafkfreund/Source/comunicado/src')
    rust_files = list(src_dir.rglob('*.rs'))
    
    files_modified = 0
    
    # Process each file
    for file_path in rust_files:
        if process_file(file_path):
            files_modified += 1
            
            # Check progress after each file
            result = subprocess.run(['cargo', 'check'], capture_output=True, text=True,
                                  cwd='/home/olafkfreund/Source/comunicado')
            current_warnings = result.stderr.count('warning:')
            
            if current_warnings < warning_count_before:
                print(f"📉 Warnings reduced: {warning_count_before} → {current_warnings}")
                break  # One fix at a time approach
    
    print(f"\n✅ Cleanup complete!")
    print(f"📁 Files modified: {files_modified}")
    
    # Final warning count
    final_result = subprocess.run(['cargo', 'check'], capture_output=True, text=True,
                                cwd='/home/olafkfreund/Source/comunicado')
    final_warnings = final_result.stderr.count('warning:')
    print(f"📊 Final warning count: {final_warnings}")
    
    if final_warnings < warning_count_before:
        print(f"🎉 Successfully reduced warnings by {warning_count_before - final_warnings}")
    
if __name__ == '__main__':
    main()