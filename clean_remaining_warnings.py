#!/usr/bin/env python3
"""
Comprehensive script to clean up remaining Rust warnings
"""
import subprocess
import re
import os
from pathlib import Path

def get_warnings():
    """Get all warnings from cargo check"""
    result = subprocess.run(['cargo', 'check', '--release', '--quiet'], 
                          capture_output=True, text=True, cwd='.')
    return result.stderr

def fix_unused_fields():
    """Fix unused field warnings by adding #[allow(dead_code)] or removing fields"""
    warnings_text = get_warnings()
    fixes_applied = 0
    
    # Pattern for unused fields: warning: field `field_name` is never read
    # --> file.rs:line:col
    field_pattern = r'warning: fields? `([^`]+)` (?:is|are) never read\s+-->\s+([^:]+):(\d+):(\d+)'
    single_field_pattern = r'warning: field `([^`]+)` is never read\s+-->\s+([^:]+):(\d+):(\d+)'
    
    all_matches = re.findall(field_pattern, warnings_text) + re.findall(single_field_pattern, warnings_text)
    
    print(f"Found {len(all_matches)} unused field warnings")
    
    # Group by file
    files_to_fix = {}
    for field_info, file_path, line_num, col_num in all_matches:
        if not file_path.startswith('src/'):
            continue
        if file_path not in files_to_fix:
            files_to_fix[file_path] = []
        fields = [f.strip() for f in field_info.split(',') if f.strip()]
        files_to_fix[file_path].extend([(field.strip('`'), int(line_num)) for field in fields])
    
    for file_path, field_info in files_to_fix.items():
        try:
            print(f"Fixing unused fields in {file_path}")
            with open(file_path, 'r') as f:
                content = f.read()
            
            # Add #[allow(dead_code)] before struct definitions with unused fields
            for field_name, line_num in field_info:
                # Find the struct that contains this field
                lines = content.split('\n')
                field_line_idx = line_num - 1
                
                # Look backwards to find the struct definition
                struct_line_idx = None
                for i in range(field_line_idx, -1, -1):
                    if re.search(r'^\s*pub\s+struct\s+\w+|^\s*struct\s+\w+', lines[i]):
                        struct_line_idx = i
                        break
                
                if struct_line_idx is not None:
                    # Check if #[allow(dead_code)] already exists
                    allow_line = struct_line_idx - 1
                    if allow_line >= 0 and '#[allow(dead_code)]' not in lines[allow_line]:
                        # Add #[allow(dead_code)] before the struct
                        indent = len(lines[struct_line_idx]) - len(lines[struct_line_idx].lstrip())
                        lines.insert(struct_line_idx, ' ' * indent + '#[allow(dead_code)]')
                        fixes_applied += 1
                        break
            
            # Write back the modified content
            with open(file_path, 'w') as f:
                f.write('\n'.join(lines))
                
        except Exception as e:
            print(f"Error fixing {file_path}: {e}")
    
    return fixes_applied

def fix_unused_methods():
    """Fix unused method warnings by adding #[allow(dead_code)]"""
    warnings_text = get_warnings()
    fixes_applied = 0
    
    # Pattern for unused methods
    method_pattern = r'warning: methods? `([^`]+)` (?:is|are) never used\s+-->\s+([^:]+):(\d+):(\d+)'
    
    matches = re.findall(method_pattern, warnings_text)
    print(f"Found {len(matches)} unused method warnings")
    
    # Group by file
    files_to_fix = {}
    for method_info, file_path, line_num, col_num in matches:
        if not file_path.startswith('src/'):
            continue
        if file_path not in files_to_fix:
            files_to_fix[file_path] = []
        methods = [m.strip().strip('`') for m in method_info.split(',') if m.strip()]
        files_to_fix[file_path].extend([(method, int(line_num)) for method in methods])
    
    for file_path, method_info in files_to_fix.items():
        try:
            print(f"Fixing unused methods in {file_path}")
            with open(file_path, 'r') as f:
                lines = f.readlines()
            
            # Add #[allow(dead_code)] before method definitions
            for method_name, line_num in method_info:
                line_idx = line_num - 1
                if line_idx < len(lines):
                    # Check if #[allow(dead_code)] already exists above
                    if line_idx > 0 and '#[allow(dead_code)]' not in lines[line_idx - 1]:
                        # Get indentation from the method line
                        method_line = lines[line_idx]
                        indent = len(method_line) - len(method_line.lstrip())
                        lines.insert(line_idx, ' ' * indent + '#[allow(dead_code)]\n')
                        fixes_applied += 1
            
            with open(file_path, 'w') as f:
                f.writelines(lines)
                
        except Exception as e:
            print(f"Error fixing {file_path}: {e}")
    
    return fixes_applied

def fix_visibility_issues():
    """Fix type visibility warnings"""
    warnings_text = get_warnings()
    fixes_applied = 0
    
    # Look for "more private than" warnings
    visibility_pattern = r'warning: type `([^`]+)` is more private than the item `([^`]+)`\s+-->\s+([^:]+):(\d+):(\d+)'
    
    matches = re.findall(visibility_pattern, warnings_text)
    print(f"Found {len(matches)} visibility warnings")
    
    for type_name, item_name, file_path, line_num, col_num in matches:
        if not file_path.startswith('src/'):
            continue
            
        try:
            print(f"Fixing visibility in {file_path}: {type_name} vs {item_name}")
            with open(file_path, 'r') as f:
                content = f.read()
            
            # Make the type public if it's used in a public item
            old_pattern = fr'\bstruct\s+{type_name}\b'
            new_replacement = f'pub struct {type_name}'
            
            if re.search(old_pattern, content) and f'pub struct {type_name}' not in content:
                content = re.sub(old_pattern, new_replacement, content)
                fixes_applied += 1
                
                with open(file_path, 'w') as f:
                    f.write(content)
                
        except Exception as e:
            print(f"Error fixing visibility in {file_path}: {e}")
    
    return fixes_applied

def main():
    """Main cleanup function"""
    print("🧹 Starting comprehensive warning cleanup...")
    
    initial_warnings = get_warnings()
    initial_count = len(re.findall(r'warning:', initial_warnings))
    print(f"📊 Initial warning count: {initial_count}")
    
    total_fixes = 0
    
    print("\n🔧 Fixing unused struct fields...")
    total_fixes += fix_unused_fields()
    
    print("\n🔧 Fixing unused methods...")
    total_fixes += fix_unused_methods()
    
    print("\n🔧 Fixing visibility issues...")
    total_fixes += fix_visibility_issues()
    
    print(f"\n✅ Applied {total_fixes} fixes")
    
    # Check final count
    final_warnings = get_warnings()
    final_count = len(re.findall(r'warning:', final_warnings))
    reduction = initial_count - final_count
    percentage = (reduction / initial_count * 100) if initial_count > 0 else 0
    
    print(f"📊 Final warning count: {final_count}")
    print(f"📈 Warnings eliminated: {reduction} ({percentage:.1f}% reduction)")
    
    if final_count > 0:
        print(f"\n⚠️  Remaining warnings that need manual review:")
        remaining_types = {}
        for warning in re.findall(r'warning: (.+)', final_warnings):
            warning_type = warning.split(' is never')[0] if ' is never' in warning else warning
            remaining_types[warning_type] = remaining_types.get(warning_type, 0) + 1
        
        for warning_type, count in sorted(remaining_types.items(), key=lambda x: x[1], reverse=True):
            print(f"  • {warning_type}: {count}")

if __name__ == "__main__":
    main()