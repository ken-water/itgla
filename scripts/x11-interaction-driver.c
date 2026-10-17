#include <X11/Xlib.h>
#include <X11/extensions/XTest.h>
#include <X11/keysym.h>
#include <string.h>
#include <unistd.h>

static Window find_named_window(Display *display, Window parent) {
    Window root;
    Window parent_return;
    Window *children = NULL;
    unsigned int child_count = 0;
    char *name = NULL;
    if (XFetchName(display, parent, &name) && name) {
        int matches = strstr(name, "ITGLA") != NULL;
        XFree(name);
        if (matches) {
            return parent;
        }
    }
    if (!XQueryTree(display, parent, &root, &parent_return, &children, &child_count)) {
        return 0;
    }
    for (unsigned int index = 0; index < child_count; index++) {
        Window match = find_named_window(display, children[index]);
        if (match) {
            if (children) {
                XFree(children);
            }
            return match;
        }
    }
    if (children) {
        XFree(children);
    }
    return 0;
}

static void pause_for_ui(void) {
    usleep(220000);
}

static void pause_for_persisted_refresh(void) {
    usleep(1200000);
}

static void click(Display *display, int x, int y) {
    XTestFakeMotionEvent(display, -1, x, y, CurrentTime);
    XTestFakeButtonEvent(display, 1, True, CurrentTime);
    XTestFakeButtonEvent(display, 1, False, CurrentTime);
    XFlush(display);
    pause_for_ui();
}

static void type_key(Display *display, KeySym symbol) {
    KeyCode key = XKeysymToKeycode(display, symbol);
    XTestFakeKeyEvent(display, key, True, CurrentTime);
    XTestFakeKeyEvent(display, key, False, CurrentTime);
    XFlush(display);
    pause_for_ui();
}

static void drag(Display *display, int start_x, int end_x, int y) {
    XTestFakeMotionEvent(display, -1, start_x, y, CurrentTime);
    XTestFakeButtonEvent(display, 1, True, CurrentTime);
    XFlush(display);
    pause_for_ui();
    for (int x = start_x + 4; x <= end_x; x += 4) {
        XTestFakeMotionEvent(display, -1, x, y, CurrentTime);
        XFlush(display);
        usleep(30000);
    }
    XTestFakeButtonEvent(display, 1, False, CurrentTime);
    XFlush(display);
    pause_for_ui();
}

static void scroll_right(Display *display, int x, int y, int steps) {
    XTestFakeMotionEvent(display, -1, x, y, CurrentTime);
    for (int step = 0; step < steps; step++) {
        XTestFakeButtonEvent(display, 7, True, CurrentTime);
        XTestFakeButtonEvent(display, 7, False, CurrentTime);
        XFlush(display);
        usleep(50000);
    }
    pause_for_ui();
}

int main(void) {
    Display *display = XOpenDisplay(NULL);
    if (!display) {
        return 2;
    }
    Window app = find_named_window(display, DefaultRootWindow(display));
    if (!app) {
        XCloseDisplay(display);
        return 3;
    }
    XSetInputFocus(display, app, RevertToParent, CurrentTime);
    XFlush(display);
    pause_for_ui();

    /* Put the caret inside Ports, insert text, then save. */
    click(display, 900, 294);
    type_key(display, XK_8);
    click(display, 810, 566);

    /* Exercise table geometry, font settings, and the non-modal guide. */
    click(display, 660, 36);
    drag(display, 187, 227, 263);
    click(display, 730, 36);
    click(display, 770, 36);
    click(display, 570, 36);
    click(display, 770, 106);

    /* Open and cancel Import, then traverse every visible main tab. */
    click(display, 900, 36);
    click(display, 700, 575);
    click(display, 205, 100);
    click(display, 85, 100);

    /* Reach the Actions column, then hide and restore the first server row. */
    scroll_right(display, 300, 300, 16);
    click(display, 580, 303);
    click(display, 620, 380);
    pause_for_persisted_refresh();
    click(display, 550, 150);
    pause_for_persisted_refresh();
    click(display, 550, 303);
    pause_for_persisted_refresh();
    click(display, 550, 150);
    pause_for_persisted_refresh();

    /* Moving away after a held resize must not keep changing the width. */
    XTestFakeMotionEvent(display, -1, 80, 200, CurrentTime);
    XFlush(display);

    XCloseDisplay(display);
    return 0;
}
