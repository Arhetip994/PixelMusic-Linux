import QtQuick
import QtQuick.Layouts
import Md3
import PixelMusic.Core

// Home: "SIMILAR TO <artist>" — the artists YouTube Music pairs with the one you play most (its "Fans also like",
// Catalog.artist(id).similar), with that artist's picture by the title, as YouTube Music's own "Similar to" shelves,
// and below them the playlists the artist is featured on. Play plays the artist's popular songs; Radio starts the
// artist's radio (their songs mixed with similar artists', as Radio on the artist page).
// The artist is the most played one of all time on this computer (Extras.stats). Until its similar artists have
// arrived the shelf has no height, so its section card stays hidden instead of showing empty.
Item {
    id: root

    property real sectionPad: 20

    property var artist: null
    property string seedName: ""
    property string seedId: ""
    readonly property var info: artist ? artist.info : ({})
    readonly property bool ready: artist !== null && artist.similar.count > 0

    implicitHeight: ready ? col.implicitHeight : 0
    visible: ready

    Component.onCompleted: {
        const top = (Extras.stats("all").topArtists ?? [])
        // a real artist id (a name stands in when the play had none; that cannot open an artist)
        const pick = top.find(a => (a.id ?? "") !== "" && a.id !== (a.title ?? "").toLowerCase())
        if (pick) {
            seedName = pick.title ?? ""
            seedId = pick.id
            artist = Catalog.artist(pick.id)
        }
    }

    ColumnLayout {
        id: col
        width: parent.width
        spacing: 12

        RowLayout {
            Layout.fillWidth: true
            spacing: 16
            Avatar {
                size: 56
                shape: "circle"
                name: root.info.name ?? root.seedName
                source: root.info.cover ?? ""
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0
                Label {
                    Layout.fillWidth: true
                    text: I18n.t("Similar to").toUpperCase()
                    type: "labelMedium"
                    weight: 600
                    color: Theme.fgSurfaceVariant
                }
                Label {
                    Layout.fillWidth: true
                    text: root.info.name ?? root.seedName
                    type: "headlineSmall"
                    weight: 600
                    elide: Text.ElideRight
                    HoverHandler { cursorShape: Qt.PointingHandCursor }
                    TapHandler { onTapped: Catalog.openArtist(root.seedId) }
                }
            }
            PillPair {
                Layout.alignment: Qt.AlignVCenter
                firstIcon: "play_arrow"
                firstText: I18n.t("Play")
                secondIcon: "radio"
                secondText: I18n.t("Artist radio")
                onFirstClicked: if (root.artist && root.artist.popular.count > 0) Player.playFrom(root.artist.popular, 0)
                // the radio of the artist's top song: the artist and similar artists (YouTube Music's artist radio)
                onSecondClicked: {
                    const song = root.artist && root.artist.popular.count > 0 ? (root.artist.popular.get(0).uri ?? "") : ""
                    const name = root.info.name ?? root.seedName
                    if (song.startsWith("youtube:track:"))
                        Radio.playStation(song, I18n.t("“{0}” radio", name))
                    else
                        Radio.playStation("artist:" + root.seedId, name)
                }
            }
        }

        ListView {
            id: strip
            Layout.fillWidth: true
            // the row reaches the section card's edges, tiles scroll out under its rounded sides
            Layout.leftMargin: -root.sectionPad
            Layout.rightMargin: -root.sectionPad
            Layout.preferredHeight: 200
            orientation: ListView.Horizontal
            SideScroll { flickable: strip }
            spacing: 24
            leftMargin: root.sectionPad
            rightMargin: root.sectionPad
            clip: true
            reuseItems: true
            cacheBuffer: 360
            boundsBehavior: Flickable.StopAtBounds
            model: root.artist ? root.artist.similar : null
            delegate: MediaTile {
                required property var model
                size: 148
                round: true
                placeholderIcon: "person"
                cover: model.cover ?? ""
                coverKind: "artist"
                coverArtist: model.name ?? ""
                title: model.name ?? ""
                subtitle: I18n.t("Artist")
                onClicked: Catalog.openArtist(model.artistId)
            }
        }

        // the playlists the artist is featured on (YouTube Music's "Featured on")
        Label {
            visible: lists.count > 0
            Layout.topMargin: 4
            text: I18n.t("Featured on")
            type: "titleMedium"
            weight: 600
        }
        ListView {
            id: lists
            visible: count > 0
            Layout.fillWidth: true
            Layout.leftMargin: -root.sectionPad
            Layout.rightMargin: -root.sectionPad
            Layout.preferredHeight: visible ? 220 : 0
            orientation: ListView.Horizontal
            SideScroll { flickable: lists }
            spacing: 24
            leftMargin: root.sectionPad
            rightMargin: root.sectionPad
            clip: true
            reuseItems: true
            cacheBuffer: 360
            boundsBehavior: Flickable.StopAtBounds
            model: root.artist ? root.artist.playlists : null
            delegate: MediaTile {
                required property var model
                size: 168
                placeholderIcon: "queue_music"
                cover: model.cover ?? ""
                coverKind: "playlist"
                coverTitle: model.name ?? model.title ?? ""
                title: model.name ?? model.title ?? ""
                subtitle: model.owner ?? model.subtitle ?? ""
                onClicked: Catalog.openUri(model.uri ?? "")
            }
        }
    }
}
