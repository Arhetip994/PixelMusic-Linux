import QtQuick
import Md3

// Two connected Material 3 Expressive buttons (a split / connected button group): 36 px tall to sit level with a
// text button such as "See all", the outer ends fully round, the inner ones 4 px, 2 px apart — as the queue header's
// group. The first is filled (primary), the second tonal (secondaryContainer). Pressing one rounds its inner corners
// a little, as the M3 Expressive button group does.
Row {
    id: root

    property string firstIcon: "play_arrow"
    property string secondIcon: "shuffle"
    property string firstText
    property string secondText
    signal firstClicked()
    signal secondClicked()

    spacing: 2

    component Segment: Item {
        id: seg
        property string icon
        property bool filled: false
        property bool first: false
        property string tip
        signal clicked()
        width: 44
        height: 36
        readonly property real outer: height / 2
        property real inner: area.pressed ? 10 : 4   // not readonly: a Behavior animates it
        Behavior on inner { NumberAnimation { duration: Theme.motion.spatialFast; easing.type: Easing.OutCubic } }
        Rectangle {
            anchors.fill: parent
            topLeftRadius: seg.first ? seg.outer : seg.inner
            bottomLeftRadius: seg.first ? seg.outer : seg.inner
            topRightRadius: seg.first ? seg.inner : seg.outer
            bottomRightRadius: seg.first ? seg.inner : seg.outer
            color: seg.filled ? Theme.primary : Theme.secondaryContainer
            Behavior on color { enabled: !Theme.transitioning; ColorAnimation { duration: Theme.motion.effectsDefault } }
        }
        Icon {
            anchors.centerIn: parent
            name: seg.icon
            size: 20
            fill: 1
            weight: 600
            color: seg.filled ? Theme.fgPrimary : Theme.fgSecondaryContainer
        }
        StateLayer {
            id: area
            anchors.fill: parent
            topLeftRadius: seg.first ? seg.outer : seg.inner
            bottomLeftRadius: seg.first ? seg.outer : seg.inner
            topRightRadius: seg.first ? seg.inner : seg.outer
            bottomRightRadius: seg.first ? seg.inner : seg.outer
            color: seg.filled ? Theme.fgPrimary : Theme.fgSecondaryContainer
            Accessible.name: seg.tip
            onClicked: seg.clicked()
        }
    }

    Segment {
        first: true
        filled: true
        width: 48
        icon: root.firstIcon
        tip: root.firstText
        onClicked: root.firstClicked()
    }
    Segment {
        icon: root.secondIcon
        tip: root.secondText
        onClicked: root.secondClicked()
    }
}
