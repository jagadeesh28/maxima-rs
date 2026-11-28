#!/usr/bin/env python3
"""
CI verification script checking for zero undefined CSS classes in maxima-web.
"""
import glob
import re
import sys

def main():
    css_path = "maxima-web/static/vendor/app.css"
    with open(css_path, "r", encoding="utf-8") as f:
        css = f.read()

    defined = set(re.findall(r"\.([A-Za-z_][\w\-]*)", css))
    used = set()

    files = ["maxima-web/static/index.html"] + glob.glob("maxima-web/static/js/*.js")
    for file_path in files:
        with open(file_path, "r", encoding="utf-8") as f:
            content = f.read()
        # 1. Static quoted attributes: class="..." or className='...'
        for m in re.findall(r'class(?:Name)?\s*=\s*["\']([^"\']+)["\']', content):
            cleaned = re.sub(r'\$\{[^}]*\}', ' ', m)
            for cls in cleaned.split():
                if cls and not cls.startswith("fa-") and not cls.endswith("-"):
                    used.add(cls)
        # 2. Template-literal attributes: class=`... ${...} ...`
        for m in re.findall(r'class(?:Name)?\s*=\s*`([^`]+)`', content):
            cleaned = re.sub(r'\$\{[^}]*\}', ' ', m)
            for cls in cleaned.split():
                if cls and not cls.startswith("fa-") and not cls.endswith("-"):
                    used.add(cls)
        # 3. classList.add / remove / toggle / contains('cls1', 'cls2')
        for args in re.findall(r'classList\.(?:add|remove|toggle|contains)\(([^)]+)\)', content):
            for cls in re.findall(r'["\']([A-Za-z_][\w\-]*)["\']', args):
                if cls and not cls.startswith("fa-"):
                    used.add(cls)

    missing = sorted([u for u in used if u not in defined and not u.startswith("fa-")])
    print(f"Total used: {len(used)}")
    print(f"Total defined: {len(defined)}")
    print(f"Missing count: {len(missing)}")
    if missing:
        print("Missing CSS classes:")
        for m in missing:
            print("  ", m)
        return 1

    print("Zero undefined CSS classes! Verification passed.")
    return 0

if __name__ == "__main__":
    sys.exit(main())
