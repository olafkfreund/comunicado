#!/usr/bin/env python3
"""
Automated script to fix Rust compilation warnings
"""
import subprocess
import re
import sys
from pathlib import Path

def get_warnings():
    """Get all warnings from cargo check"""
    result = subprocess.run(['cargo', 'check', '--release'], 
                          capture_output=True, text=True, cwd='.')
    return result.stderr

def fix_unused_imports(warnings_text):
    """Fix unused import warnings"""
    fixes_applied = 0
    
    # Pattern: warning: unused import: `ImportName`
    #   --> file.rs:line:col
    import_pattern = r'warning: unused import: `([^`]+)`\s+-->\s+([^:]+):(\d+):(\d+)'
    
    matches = re.findall(import_pattern, warnings_text)
    print(f"Found {len(matches)} unused import warnings")
    
    # Group by file to batch fixes
    files_to_fix = {}
    for import_name, file_path, line_num, col_num in matches:
        if file_path not in files_to_fix:
            files_to_fix[file_path] = []
        files_to_fix[file_path].append((import_name, int(line_num)))
    
    for file_path, imports in files_to_fix.items():
        if not file_path.startswith('src/'):
            continue
            
        try:
            print(f"Fixing unused imports in {file_path}")
            with open(file_path, 'r') as f:
                lines = f.readlines()
            
            # Sort by line number descending to avoid index issues
            imports.sort(key=lambda x: x[1], reverse=True)
            
            for import_name, line_num in imports:
                line_idx = line_num - 1
                if line_idx < len(lines):
                    line = lines[line_idx]
                    
                    # Check if it's a single import on its own line
                    if f'use ' in line and import_name in line:
                        # If it's a single import, comment it out
                        if line.strip().endswith(f'{import_name};'):
                            lines[line_idx] = f"// {line}"
                            fixes_applied += 1
                        # If it's part of a multi-import, remove just that import
                        elif '{' in line and '}' in line:
                            # Remove the specific import from the list
                            new_line = line.replace(f', {import_name}', '').replace(f'{import_name}, ', '').replace(f'{{{import_name}}}', '{}')
                            if new_line != line:
                                lines[line_idx] = new_line
                                fixes_applied += 1
            
            with open(file_path, 'w') as f:
                f.writelines(lines)
                
        except Exception as e:
            print(f"Error fixing {file_path}: {e}")
    
    return fixes_applied

def fix_unused_variables(warnings_text):
    """Fix unused variable warnings by prefixing with underscore"""
    fixes_applied = 0
    
    # Pattern: warning: unused variable: `var_name`
    var_pattern = r'warning: unused variable: `([^`]+)`\s+-->\s+([^:]+):(\d+):(\d+)'
    
    matches = re.findall(var_pattern, warnings_text)
    print(f"Found {len(matches)} unused variable warnings")
    
    # Group by file
    files_to_fix = {}
    for var_name, file_path, line_num, col_num in matches:
        if file_path not in files_to_fix:
            files_to_fix[file_path] = []
        files_to_fix[file_path].append((var_name, int(line_num)))
    
    for file_path, variables in files_to_fix.items():
        if not file_path.startswith('src/'):
            continue
            
        try:
            print(f"Fixing unused variables in {file_path}")
            with open(file_path, 'r') as f:
                content = f.read()
            
            for var_name, line_num in variables:
                # Replace variable declarations with underscore prefix
                patterns = [
                    fr'\blet {var_name}\b',
                    fr'\blet mut {var_name}\b',
                    fr'\|{var_name}\|',
                    fr'\|{var_name},',
                    fr', {var_name}\|',
                    fr'for {var_name} in',
                ]
                
                for pattern in patterns:
                    if re.search(pattern, content):
                        content = re.sub(pattern, pattern.replace(var_name, f'_{var_name}'), content)
                        fixes_applied += 1
                        break
            
            with open(file_path, 'w') as f:
                f.write(content)
                
        except Exception as e:
            print(f"Error fixing variables in {file_path}: {e}")
    
    return fixes_applied

def main():
    """Main function to fix warnings"""
    print("🔧 Starting automated warning fixes...")
    
    warnings = get_warnings()
    if not warnings:
        print("✅ No warnings found!")
        return
    
    total_fixes = 0
    
    print("\n📦 Fixing unused imports...")
    total_fixes += fix_unused_imports(warnings)
    
    print("\n🔀 Fixing unused variables...")  
    total_fixes += fix_unused_variables(warnings)
    
    print(f"\n✅ Applied {total_fixes} automatic fixes")
    
    # Check remaining warnings
    print("\n🔍 Checking remaining warnings...")
    new_warnings = get_warnings()
    remaining_count = len(re.findall(r'warning:', new_warnings))
    print(f"📊 Remaining warnings: {remaining_count}")
    
    if remaining_count > 0:
        print("\n⚠️  Remaining warnings that need manual fixes:")
        print(new_warnings)

if __name__ == "__main__":
    main()