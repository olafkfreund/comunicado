#!/usr/bin/env python3
"""
Fix unused import warnings in Rust code by commenting them out.
"""

import re
import subprocess
import os

def get_unused_warnings():
    """Get all unused import warnings from cargo check."""
    result = subprocess.run(['cargo', 'check'], capture_output=True, text=True, cwd='.')
    warnings = []
    
    current_file = None
    current_line = None
    
    for line in result.stderr.split('\n'):
        # Look for file and line info
        if ' --> ' in line and ':' in line:
            parts = line.split(' --> ')[1].split(':')
            if len(parts) >= 2:
                current_file = parts[0]
                current_line = int(parts[1])
        
        # Look for unused import warnings
        if 'unused import' in line and current_file and current_line:
            # Extract the import names
            if 'unused imports:' in line:
                imports = line.split('unused imports:')[1].strip()
                # Remove backticks and quotes  
                imports = re.sub(r'`([^`]+)`', r'\1', imports)
                warnings.append({
                    'file': current_file,
                    'line': current_line,
                    'imports': imports,
                    'type': 'multiple'
                })
            elif 'unused import:' in line:
                import_name = line.split('unused import:')[1].strip()
                import_name = re.sub(r'`([^`]+)`', r'\1', import_name)
                warnings.append({
                    'file': current_file,
                    'line': current_line, 
                    'imports': import_name,
                    'type': 'single'
                })
    
    return warnings

def fix_unused_import(file_path, line_num):
    """Comment out an unused import line."""
    try:
        with open(file_path, 'r') as f:
            lines = f.readlines()
        
        if line_num <= len(lines):
            line = lines[line_num - 1]  # Convert to 0-based index
            if not line.strip().startswith('//'):
                # Comment out the line
                lines[line_num - 1] = '// ' + line
                
                with open(file_path, 'w') as f:
                    f.writelines(lines)
                return True
    except Exception as e:
        print(f"Error fixing {file_path}:{line_num}: {e}")
    
    return False

def main():
    warnings = get_unused_warnings()
    
    if not warnings:
        print("No unused import warnings found!")
        return
    
    print(f"Found {len(warnings)} unused import warnings")
    
    fixed_count = 0
    for warning in warnings:
        file_path = warning['file']
        line_num = warning['line']
        
        if os.path.exists(file_path):
            if fix_unused_import(file_path, line_num):
                print(f"Fixed: {file_path}:{line_num}")
                fixed_count += 1
            else:
                print(f"Skipped: {file_path}:{line_num} (already commented or error)")
        else:
            print(f"File not found: {file_path}")
    
    print(f"\nFixed {fixed_count} unused import warnings")
    
    # Run cargo check again to verify
    print("\nVerifying fixes...")
    result = subprocess.run(['cargo', 'check'], capture_output=True, text=True)
    remaining_warnings = len([line for line in result.stderr.split('\n') if 'unused import' in line])
    print(f"Remaining unused import warnings: {remaining_warnings}")

if __name__ == '__main__':
    main()