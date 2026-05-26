export const friends = [
  {
    id: 1,
    username: "Jack",
    online: true,
    avatar: "src//img/profilepicture.png",
    unread: 2,

    messages: [
      {
        text: "Hi, do you wanna play something in a bit?",
        own: false,
        time: "19:20"
      },

      {
        text: "Hey sure, just give me some minutes 💪🏽",
        own: true,
        time: "19:21"
      }
    ]
  },

  {
    id: 2,
    username: "Simon",
    online: false,
    avatar: "src//img/profilepicture.png",
    unread: 0,

    messages: [
      {
        text: "I will call you later!",
        own: false,
        time: "20:10"
      }
    ]
  }
];