import { FishId, Pond, Fish, Tag } from '@actyx/pond'

type ChatEvent = string
type ChatState = ChatEvent[]

const INITIAL_STATE: ChatState = []

function onEvent(state: ChatState, event: ChatEvent) {
    state.push(event);
    return state;
}

const chatTag = Tag<ChatEvent>('ChatMessage')
const ChatFish: Fish<ChatState, ChatEvent> = ({
    fishId: FishId.of('ax.example.chat', 'MyChatFish', 0),
    initialState: INITIAL_STATE,
    onEvent: onEvent,
    where: chatTag
})

Pond.default().then(pond => {
    // Select UI elements in the DOM
    const messagesTextArea = document.getElementById('messages')
    const messageInput = <HTMLInputElement>document.getElementById('message')
    const sendButton = document.getElementById('send')

    function clearInputAndSendToStream() {
        // When click on send button get the text written in the input field
        const message = messageInput.value
        messageInput.value = ''
        // Send the message to a stream tagged with our chat tag
        pond.emit(chatTag, message)
    }

    sendButton.addEventListener('click', clearInputAndSendToStream)

    // Observe our chat fish. This means that our callback function will
    // be called anytime the state of the fish changes
    pond.observe(ChatFish, state => {
        // Get the `pre` element and add all chat messages to that element
        messagesTextArea.innerHTML = state.join('\n')
        // Scroll the element to the bottom when it is updated
        messagesTextArea.scrollTop = messagesTextArea.scrollHeight
    });
}).catch(console.log)