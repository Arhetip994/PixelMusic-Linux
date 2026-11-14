import QtQuick

// Переключатель MD3 с иконками в бегунке.
Item {
    id: root

    property bool checked: false
    property bool icons: true
    readonly property alias pressed: area.pressed
    // colours: the theme by default; a branded card (the YouTube account) sets its own
    property color trackOn: Theme.primary
    property color trackOff: Theme.surfaceContainerHighest
    property color outlineOff: Theme.outline
    property color thumbOn: Theme.fgPrimary
    property color thumbOff: Theme.outline
    property color iconOn: trackOn
    property color iconOff: trackOff

    signal toggled(bool checked)

    implicitWidth: 52
    implicitHeight: 32

    Rectangle {
        id: track
        anchors.fill: parent
        radius: height / 2
        color: root.checked ? root.trackOn : root.trackOff
        border.width: root.checked ? 0 : 2
        border.color: root.outlineOff
        opacity: root.enabled ? 1 : Theme.stateLayer.disabledContainer
        Behavior on color { enabled: !Theme.transitioning; ColorAnimation { duration: Theme.motion.effectsDefault } }
    }

    Item {
        id: thumbHost
        width: 40
        height: 40
        anchors.verticalCenter: parent.verticalCenter
        x: (root.checked ? root.width - root.height / 2 : root.height / 2) - width / 2
        Behavior on x { NumberAnimation { duration: Theme.motion.spatialFast; easing.type: Easing.OutBack; easing.overshoot: 1.6 } }

        // слой состояния вокруг бегунка
        Rectangle {
            anchors.fill: parent
            radius: width / 2
            color: root.checked ? root.trackOn : Theme.fgSurface
            opacity: area.pressed ? Theme.stateLayer.pressed : area.containsMouse ? Theme.stateLayer.hover : 0
            Behavior on opacity { NumberAnimation { duration: Theme.motion.effectsFast } }
        }

        Rectangle {
            id: thumb
            property real d: area.pressed ? 28 : (root.checked || root.icons) ? 24 : 16
            width: d
            height: d
            radius: d / 2
            anchors.centerIn: parent
            color: root.checked ? root.thumbOn : root.thumbOff
            Behavior on d { NumberAnimation { duration: Theme.motion.spatialFast; easing.type: Easing.OutBack } }
            Behavior on color { enabled: !Theme.transitioning; ColorAnimation { duration: Theme.motion.effectsDefault } }

            Icon {
                anchors.centerIn: parent
                visible: root.icons
                name: root.checked ? "check" : "close"
                size: 16
                weight: 600
                color: root.checked ? root.iconOn : root.iconOff  // fgPrimaryContainer в схеме 2025 сливается с бегунком
            }
        }
    }

    MouseArea {
        id: area
        anchors.fill: parent
        anchors.margins: -4
        hoverEnabled: true
        enabled: root.enabled
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            root.checked = !root.checked
            root.toggled(root.checked)
        }
    }
}
