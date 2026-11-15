import QtQuick
import QtQuick.Layouts
import Md3

// Placeholder track rows while a list loads (search results, an artist's tracks, a Home list shelf): the shape of a
// TrackRow (64 px, a 48 px cover, title and artist bars, the duration), softly pulsing, in one or two columns, so the
// real rows land where the placeholders stood instead of the page jumping when they arrive.
Item {
    id: root
    property int rows: 6
    property int columns: 1
    property bool numbered: false
    property color tone: Theme.surfaceContainerHigh
    // off inside a placeholder that already pulses as a whole (Home's skeleton)
    property bool pulse: true
    readonly property real rowHeight: 64

    implicitWidth: 320
    implicitHeight: grid.implicitHeight

    SequentialAnimation on opacity {
        running: root.pulse && root.visible && !Theme.reducedMotion && !Theme.quiet
        loops: Animation.Infinite
        NumberAnimation { to: 0.55; duration: 700; easing.type: Easing.InOutSine }
        NumberAnimation { to: 1; duration: 700; easing.type: Easing.InOutSine }
    }

    Grid {
        id: grid
        width: parent.width
        columns: Math.max(1, root.columns)
        columnSpacing: 16
        Repeater {
            model: root.rows
            Item {
                id: row
                required property int index
                width: (grid.width - grid.columnSpacing * (grid.columns - 1)) / grid.columns
                height: root.rowHeight
                // bars of slightly different lengths, so the list does not look stamped
                readonly property real vary: [0.9, 0.7, 0.8, 0.6, 0.85, 0.75][index % 6]
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    spacing: 12
                    Rectangle {
                        visible: root.numbered
                        Layout.preferredWidth: 14
                        Layout.preferredHeight: 12
                        radius: 6
                        color: root.tone
                    }
                    Rectangle {
                        Layout.preferredWidth: 48
                        Layout.preferredHeight: 48
                        radius: 12
                        color: root.tone
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        Rectangle {
                            Layout.preferredWidth: Math.min(220, parent.width) * row.vary
                            Layout.preferredHeight: 12
                            radius: 6
                            color: root.tone
                        }
                        Rectangle {
                            Layout.preferredWidth: Math.min(150, parent.width) * row.vary
                            Layout.preferredHeight: 10
                            radius: 5
                            color: root.tone
                        }
                    }
                    Rectangle {
                        Layout.preferredWidth: 32
                        Layout.preferredHeight: 10
                        radius: 5
                        color: root.tone
                    }
                }
            }
        }
    }
}
