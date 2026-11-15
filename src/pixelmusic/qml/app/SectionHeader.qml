import QtQuick
import QtQuick.Layouts
import Md3

// Заголовок секции страницы с необязательной кнопкой справа («Все», «Свернуть»).
RowLayout {
    id: root

    property string text
    property string actionText
    // a song shelf's songs: Play and Shuffle at the end of the row, after the action
    property var tracks: []
    signal action()

    Layout.fillWidth: true
    Layout.topMargin: 8
    spacing: 12

    Label {
        Layout.fillWidth: true
        text: root.text
        type: "headlineSmall"
        weight: 600
    }
    // "See all" first, then the connected Play | Shuffle pair at the row's end
    Button {
        visible: root.actionText !== ""
        text: root.actionText
        style: "text"
        onClicked: root.action()
    }
    PlayShuffleButtons {
        Layout.alignment: Qt.AlignVCenter
        tracks: root.tracks
    }
}
