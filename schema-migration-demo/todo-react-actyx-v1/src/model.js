import { FishId, Tag, Tags } from "@actyx/pond";

export const taskEdited = (id, name) => [
    // tags ascribe meaning: this is about the task of the given ID
    Tags(`task:${id}`),
    {
        type: 'taskEdited',
        id,
        name
    }
]

export const taskAdded = (id, name) => [
    // currently tags also double for indexing, here to mark relevance for taskList
    // (future ActyxOS versions will have better indexing capabilities)
    Tags(`task:${id}`, 'task-list'),
    {
        type: 'taskAdded',
        id,
        name
    }
]

export const taskDeleted = (id) => [
    Tags(`task:${id}`, 'task-list'),
    {
        type: 'taskDeleted',
        id
    }
]

export const taskStatus = (id, completed) => [
    Tags(`task:${id}`),
    {
        type: 'taskStatus',
        // events are self-contained facts, which is why the ID is here — who knows which future code will need it?
        id,
        completed
    }
]

export const taskListFish = {
    // the trailing 0 marks onEvent code version for state snapshot management
    fishId: FishId.of('taskList', 'singleton', 0),
    initialState: [],
    where: Tag('task-list'),
    onEvent: (state, event, _meta) => {
        switch (event.type) {
            case 'taskAdded':
                state.push(event.id)
                break
            case 'taskDeleted':
                const pos = state.indexOf(event.id)
                if (pos !== -1) { state.splice(pos, 1) }
                break
            default:
        }
        return state
    }
}

export const taskDetailsFish = (id) => ({
    fishId: FishId.of('taskDetails', id, 0),
    initialState: { id, completed: false },
    where: Tag(`task:${id}`),
    onEvent: (state, event, _meta) => {
        switch (event.type) {
            case 'taskAdded':
                state.name = event.name
                break
            case 'taskEdited':
                state.name = event.name
                break
            case 'taskStatus':
                state.completed = event.completed
                break
            default:
        }
        return state
    }
})