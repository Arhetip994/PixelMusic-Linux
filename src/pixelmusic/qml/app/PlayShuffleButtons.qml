import QtQuick
import Md3
import PixelMusic.Core

// Play and Shuffle for a list of songs (Home's song shelves, a shelf's "See all" page): one connected pair of
// Material 3 Expressive buttons (PillPair), Play filled and Shuffle tonal, the height of a text button. Play starts
// the list at its first song; Shuffle plays a shuffled copy of it (the player's own shuffle setting is left as it
// is). While this very list is playing, Play pauses and resumes it instead of starting over.
Row {
    id: root

    property var tracks: []

    spacing: 8
    visible: (tracks ?? []).length > 0

    // the song playing is one of these (the list was started here, or the same songs are queued)
    readonly property bool _playingHere: Player.hasTrack && Player.trackId !== ""
                                         && (tracks ?? []).some(t => (t.trackId ?? "") === Player.trackId)

    function play() {
        if (root._playingHere) {
            Player.togglePlay()
            return
        }
        Player.rememberTrack(root.tracks[0])
        Player.playTracks(root.tracks, 0)
    }
    function shuffle() {
        const list = root.tracks.slice()
        for (let i = list.length - 1; i > 0; i--) {
            const j = Math.floor(Math.random() * (i + 1))
            const t = list[i]
            list[i] = list[j]
            list[j] = t
        }
        Player.rememberTrack(list[0])
        Player.playTracks(list, 0)
    }

    PillPair {
        firstIcon: root._playingHere && Player.playing ? "pause" : "play_arrow"
        firstText: root._playingHere && Player.playing ? I18n.t("Pause") : I18n.t("Play")
        secondIcon: "shuffle"
        secondText: I18n.t("Shuffle play")
        onFirstClicked: root.play()
        onSecondClicked: root.shuffle()
    }
}
