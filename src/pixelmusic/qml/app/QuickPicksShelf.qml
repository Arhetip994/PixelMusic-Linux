import QtQuick
import QtQuick.Layouts
import Md3
import PixelMusic.Core

// Quick Picks shelf - 2-column grid of tracks with cover, title, artist, and duration
// Similar to the SoundWave Android app's Quick Picks section
ColumnLayout {
    id: root

    property var tracks: []
    property bool compact: false
    property string title: I18n.t("Quick picks")
    property bool showMore: false
    // "Show more" opens the rest of the shelf in place (the first 8 songs otherwise)
    property bool expanded: false

    signal activated(var track, int index)
    signal showMoreClicked()
    signal trackMenuRequested(var track, point position)

    spacing: 12

    // Section header with optional "Show more" action
    RowLayout {
        Layout.fillWidth: true
        Layout.topMargin: 8
        spacing: 8

        Label {
            Layout.fillWidth: true
            text: root.title
            type: "headlineSmall"
            weight: 600
            fade: true
        }

        Label {
            visible: root.showMore
            Layout.alignment: Qt.AlignRight
            text: root.expanded ? I18n.t("Show less") : I18n.t("Show more")
            type: "labelLarge"
            color: Theme.primary

            HoverHandler { id: moreHover; cursorShape: Qt.PointingHandCursor }

            Rectangle {
                anchors.fill: parent
                anchors.margins: -4
                radius: Theme.shape.small
                color: Theme.primary
                opacity: moreHover.hovered ? 0.1 : 0
            }

            TapHandler {
                onTapped: { root.expanded = !root.expanded; root.showMoreClicked() }
            }
        }
    }

    // 2-column grid of tracks
    GridLayout {
        id: grid
        Layout.fillWidth: true
        columns: root.compact ? 1 : 2
        columnSpacing: 16
        rowSpacing: 8

        Repeater {
            model: root.showMore && !root.expanded ? root.tracks.slice(0, 8) : root.tracks

            Item {
                id: trackItem
                required property var modelData
                required property int index

                readonly property string trackId: modelData.trackId ?? ""
                readonly property string title: modelData.title ?? ""
                readonly property string artists: modelData.artists ?? ""
                readonly property string cover: modelData.cover ?? ""
                readonly property int durationMs: modelData.durationMs ?? 0
                readonly property bool current: trackId !== "" && trackId === Player.trackId
                readonly property bool playing: current && Player.playing

                Layout.fillWidth: true
                Layout.preferredHeight: 56

                // the playing song: a solid primaryContainer pill, fully rounded (the queue's current card)
                readonly property color fg: current ? Theme.fgPrimaryContainer : Theme.fgSurface
                readonly property color fgSoft: current ? Qt.alpha(Theme.fgPrimaryContainer, 0.75) : Theme.fgSurfaceVariant
                Rectangle {
                    id: pickBg
                    anchors.fill: parent
                    radius: trackItem.current ? height / 2 : Theme.shape.large
                    color: trackItem.current ? Theme.primaryContainer : Qt.alpha(Theme.primaryContainer, 0)
                    Behavior on color { enabled: !Theme.transitioning; ColorAnimation { duration: Theme.motion.effectsDefault } }
                    Behavior on radius { enabled: !Theme.reducedMotion; NumberAnimation { duration: Theme.motion.spatialDefault; easing.type: Easing.BezierSpline; easing.bezierCurve: Theme.motion.emphasized } }
                }

                // Hover effect
                Rectangle {
                    anchors.fill: parent
                    radius: pickBg.radius
                    color: trackItem.fg
                    opacity: area.pressed ? Theme.stateLayer.pressed : area.containsMouse ? Theme.stateLayer.hover : 0

                    Behavior on opacity { NumberAnimation { duration: Theme.motion.effectsFast } }
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    spacing: 12

                    // Track number or current indicator
                    Item {
                        Layout.preferredWidth: 28
                        Layout.preferredHeight: 28

                        Label {
                            visible: !trackItem.current
                            anchors.centerIn: parent
                            text: trackItem.index + 1
                            type: "bodyMedium"
                            color: trackItem.fgSoft
                            horizontalAlignment: Text.AlignHCenter
                        }

                        // the queue's playing bars (QueueRow): moving while it plays, folded to dots when paused
                        PlayingEqIcon {
                            visible: trackItem.current
                            anchors.centerIn: parent
                            width: 20
                            height: 18
                            color: trackItem.fg
                            playing: Player.playing
                        }
                    }

                    // Track cover
                    Item {
                        Layout.preferredWidth: 44
                        Layout.preferredHeight: 44

                        MorphImage {
                            anchors.fill: parent
                            source: trackItem.cover
                            shape: "softSquare"
                            fallbackKind: "track"
                            fallbackArtist: trackItem.artists.split(", ")[0]
                            fallbackTitle: trackItem.title
                        }
                    }

                    // Track info (title and artist)
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        Label {
                            Layout.fillWidth: true
                            text: trackItem.title
                            type: "titleSmall"
                            color: trackItem.fg
                            fade: true
                            elide: Text.ElideRight
                        }

                        Label {
                            Layout.fillWidth: true
                            text: trackItem.artists
                            type: "bodySmall"
                            color: trackItem.fgSoft
                            fade: true
                            elide: Text.ElideRight
                        }
                    }

                    // the backend fills lengths in; unknown — nothing
                    Label {
                        id: durationLabel
                        text: trackItem.durationMs > 0 ? root._formatTime(trackItem.durationMs) : ""
                        type: "bodySmall"
                        color: trackItem.fgSoft
                        horizontalAlignment: Text.AlignRight
                        Layout.preferredWidth: 40
                    }
                }

                MouseArea {
                    id: area
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    acceptedButtons: Qt.LeftButton | Qt.RightButton

                    onClicked: mouse => {
                        if (mouse.button === Qt.RightButton) {
                            const p = mapToItem(null, mouse.x, mouse.y)
                            root.trackMenuRequested(trackItem.modelData, p)
                        } else {
                            root.activated(trackItem.modelData, trackItem.index)
                        }
                    }
                }

                // drag the song into the queue (QueueView, the mini player's queue button): they read trackData()
                function trackData() { return trackItem.modelData }
                // under the row's MouseArea (z -1): above it, it swallowed every tap
                QueueDragSource {
                    anchors.fill: parent
                    z: -1
                    source: trackItem
                    enabled: (trackItem.modelData.uri ?? "") !== "" || trackItem.trackId !== ""
                    title: trackItem.title
                    cover: trackItem.cover
                }
            }
        }
    }

    function _formatTime(ms) {
        const s = Math.round(ms / 1000)
        return Math.floor(s / 60) + ":" + String(s % 60).padStart(2, "0")
    }
}
