import QtQuick
import QtQuick.Layouts
import Md3

// Заглушка карточки (обложка + две строки), пока нет данных.
ColumnLayout {
    id: root
    property var shape: "softSquare"
    property real size: 156
    // inside a section card the placeholders sit one tone up (surfaceContainerHighest), like the real tiles
    property color tone: Theme.surfaceContainerHigh

    spacing: 10
    MorphShape {
        Layout.preferredWidth: root.size
        Layout.preferredHeight: root.size
        shape: root.shape
        color: root.tone
    }
    Rectangle { Layout.preferredWidth: root.size * 0.8; Layout.preferredHeight: 12; radius: 6; color: root.tone }
    Rectangle { Layout.preferredWidth: root.size * 0.5; Layout.preferredHeight: 10; radius: 5; color: root.tone }
}
