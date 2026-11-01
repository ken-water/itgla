#!/usr/bin/env python3
"""Set complete Debian control fields without discarding packager metadata."""
import sys
from pathlib import Path


def update(path, dependencies):
    text = path.read_text()
    lines = text.splitlines()
    fields = {}
    current = None
    for line in lines:
        if line.startswith((' ', '\t')) and current:
            fields[current] += '\n' + line
        elif ':' in line:
            current, value = line.split(':', 1)
            fields[current] = value.lstrip()
    fields['Maintainer'] = 'ITGLA <service@itgla.com>'
    # Winit loads display libraries dynamically, so ELF scanning alone omits them.
    display = 'libxkbcommon0, libxkbcommon-x11-0, libx11-6, libx11-xcb1, libxcb1, libxcursor1, libxi6, libxrandr2'
    fields['Depends'] = dependencies + ', ' + display
    fields['Recommends'] = 'xdg-desktop-portal, xdg-desktop-portal-gtk | xdg-desktop-portal-kde'
    fields['Description'] = ('IT in a glance - local IT asset workspace\n'
                             ' Manage servers, services, domains and their relationships locally.')
    path.write_text(''.join(f'{name}: {value}\n' for name, value in fields.items()))


if __name__ == '__main__':
    update(Path(sys.argv[1]), sys.argv[2])
