import Gio from 'gi://Gio';
import Meta from 'gi://Meta';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';

const BUS_NAME = 'casa.tafu.tandem.WindowTargeting';
const OBJECT_PATH = '/casa/tafu/tandem/WindowTargeting';

const WindowTargetingIface = `
<node>
  <interface name="casa.tafu.tandem.WindowTargeting">
    <method name="Ping">
      <arg type="b" name="alive" direction="out"/>
    </method>
    <method name="ListWindows">
      <arg type="s" name="windows" direction="out"/>
    </method>
    <method name="ActivateWindow">
      <arg type="s" name="id" direction="in"/>
      <arg type="b" name="success" direction="out"/>
    </method>
  </interface>
</node>`;

class WindowTargetingDBus {
    Ping() {
        return true;
    }

    ListWindows() {
        const windows = [];

        for (const actor of global.get_window_actors()) {
            const metaWindow = actor.meta_window;
            if (!metaWindow) {
                continue;
            }

            if (metaWindow.is_override_redirect()) {
                continue;
            }

            if (metaWindow.get_window_type() !== Meta.WindowType.NORMAL) {
                continue;
            }

            const title = metaWindow.get_title()?.trim() ?? '';
            if (!title) {
                continue;
            }

            const app =
                metaWindow.get_gtk_application_id() ||
                metaWindow.get_wm_class() ||
                '';

            windows.push({
                id: String(metaWindow.get_id()),
                title,
                app,
            });
        }

        return JSON.stringify(windows);
    }

    ActivateWindow(id) {
        const targetId = Number.parseInt(id, 10);
        if (!Number.isFinite(targetId)) {
            return false;
        }

        for (const actor of global.get_window_actors()) {
            const metaWindow = actor.meta_window;
            if (!metaWindow || metaWindow.get_id() !== targetId) {
                continue;
            }

            Main.activateWindow(metaWindow);
            return true;
        }

        return false;
    }
}

export default class TandemWindowTargetingExtension extends Extension {
    enable() {
        this._impl = new WindowTargetingDBus();
        this._dbus = Gio.DBusExportedObject.wrapJSObject(WindowTargetingIface, this._impl);
        this._dbus.export(Gio.DBus.session, OBJECT_PATH);
        this._ownerId = Gio.DBus.session.own_name(
            BUS_NAME,
            Gio.BusNameOwnerFlags.NONE,
            null,
            null,
        );
    }

    disable() {
        if (this._ownerId) {
            Gio.DBus.session.unown_name(this._ownerId);
            this._ownerId = null;
        }

        if (this._dbus) {
            this._dbus.unexport();
            this._dbus = null;
        }

        this._impl = null;
    }
}
