import { Layout } from "../components/Layout"
import React, { useEffect, useMemo, useState } from "react"
import { Cell, ColWidth, Row, TruncatableString } from "./Query"
import { useAppState } from "../app-state"
import { TopicDeleteResponse, TopicLsResponse } from "common/types"
import clsx from "clsx"
import { Button } from "../components/basics"
import { Either, isLeft, isRight, left, right } from "fp-ts/Either"


function bytesToMegabytes(bytes: number) {
    return bytes / 1e6
}

const LsHeaderRow = () => {
    const cells: [string, ColWidth | undefined][] = [
        ["Node", "96"],
        ["Topic", "32"],
        ["Size (MB)", "32"],
        ["Active", undefined]
    ]
    return (
        <Row
            height="8"
            isChecked={false}
            backgroundColor="gray"
            textColor="gray"
            className="font-bold border-t rounded-t-md"

        >
            {() => cells.map(([text, width]) => (
                <Cell
                    key={text}
                    height="8"
                    width={width}
                    rowIsExpanded={false}
                    isLast={text === "Active"}
                >
                    <TruncatableString>{text}</TruncatableString>
                </Cell>
            ))}
        </Row >
    )
}

const LsResultRow = ({ topic, toggle, isChecked }: { topic: FlatTopic, toggle: () => void, isChecked: boolean }) => {
    const cells: [string, string, ColWidth | undefined][] = [
        ["node", topic.nodeId, "96"],
        ["topic", topic.name, "32"],
        ["size", bytesToMegabytes(topic.size).toFixed(2), "32"],
        ["active", topic.isActive ? "✓" : "", undefined]
    ]
    const jsxCells = cells.map(([keyPrefix, text, width]) => (
        <Cell
            key={`${keyPrefix}+${text}`}
            height="7"
            width={width}
            className={clsx({ 'font-mono': (keyPrefix === 'node' || keyPrefix == "size") })}
            isLast={keyPrefix === "active"}
            rowIsExpanded={false}
        >
            <TruncatableString>{text}</TruncatableString>
        </Cell>
    ))

    if (topic.isActive) {
        return <Row
            height="7"
            isChecked={false}
        >
            {() => jsxCells}
        </Row>
    } else {
        return <Row
            height="7"
            isChecked={isChecked}
            onChecked={toggle}
            onUnchecked={toggle}
        >
            {() => jsxCells}
        </Row>
    }
}

const ErrorRow = ({ error }: { error: string }) => {
    return (
        <Row
            height="7"
            isChecked={false}
        >
            {() => (<Cell
                key={`error+${error}`}
                height="7"
                className={clsx('font-mono')}
                isLast={true}
                rowIsExpanded={false}
            >
                <TruncatableString>{error}</TruncatableString>
            </Cell>)}
        </Row>
    )
}

type FlatTopic = {
    nodeAddress: string,
    nodeId: string,
    isActive: boolean,
    name: string,
    size: number
}

type TopicLsResponseWithAddress = { nodeAddress: string } & TopicLsResponse

function fromTopicLs(response: TopicLsResponseWithAddress): FlatTopic[] {
    return Object.entries(response.topics).map(
        ([name, size]) => ({
            nodeAddress: response.nodeAddress,
            nodeId: response.nodeId,
            isActive: name === response.activeTopic,
            name,
            size
        })
    )
}

const LsResults = ({ topics, checkedIxs, toggle }: { topics: Either<{ nodeAddr: string, error: string }, FlatTopic>[], checkedIxs: Set<number>, toggle: (ix: number) => void }) => {
    const results = topics.map((topic, ix) => {
        if (isLeft(topic)) {
            return <ErrorRow error={topic.left.error}></ErrorRow>
        } else {
            return <LsResultRow topic={topic.right} toggle={() => toggle(ix)} isChecked={checkedIxs.has(ix)} ></ LsResultRow>
        }
    })
    return <div className="flex-grow mt-6 border-b border-l border-r rounded-md mb-1 text-xs flex flex-col">
        <LsHeaderRow></LsHeaderRow>
        <div className="flex-grow flex-shrink h-1 overflow-y-scroll overflow-x-hidden">
            {results}
        </div>
    </div>
}


const DeleteHeaderRow = () => {
    const cells: [string, ColWidth | undefined][] = [
        ["Node", "96"],
        ["Topic", "32"],
        ["Deleted", undefined]
    ]
    return (
        <Row
            height="8"
            isChecked={false}
            backgroundColor="gray"
            textColor="gray"
            className="font-bold border-t rounded-t-md"
        >
            {() => cells.map(([text, width]) => (
                <Cell
                    key={text}
                    height="8"
                    width={width}
                    rowIsExpanded={false}
                    isLast={text === "Deleted"}
                >
                    <TruncatableString>{text}</TruncatableString>
                </Cell>
            ))
            }
        </Row >
    )
}

type DeletedTopic = { name: string } & TopicDeleteResponse

const DeleteResultRow = ({ deleted }: { deleted: DeletedTopic }) => {
    const cells: [string, string, ColWidth | undefined][] = [
        ["node", deleted.nodeId, "96"],
        ["topic", deleted.name, "32"],
        ["deleted", deleted.deleted ? "✓" : "", undefined]
    ]
    const jsxCells = cells.map(([keyPrefix, text, width]) => (
        <Cell
            key={`${keyPrefix}+${text}`}
            height="7"
            width={width}
            className={clsx({ 'font-mono': keyPrefix === 'node' })}
            isLast={keyPrefix === "active"}
            rowIsExpanded={false}
        >
            <TruncatableString>{text}</TruncatableString>
        </Cell>
    ))

    return (
        <Row
            height="7"
            isChecked={false}
        >
            {() => jsxCells}
        </Row>
    )
}

const Screen: React.FC<{}> = () => {
    const {
        data: { nodes },
        actions: { getTopicList, deleteTopic }
    } = useAppState()

    const [checkedIxs, setCheckedIxs] = useState<Set<number>>(new Set())
    const [topics, setTopics] = useState<Either<{ nodeAddr: string, error: string }, FlatTopic>[]>([]);
    const [deletedTopics, setDeletedTopics] = useState<DeletedTopic[]>([])

    const fetchTopics: () => Promise<Either<{ nodeAddr: string, error: string }, TopicLsResponseWithAddress>[]> = async () => {
        let topics = await Promise.all(
            nodes.map(async (node) => {
                try {
                    const res = await getTopicList(node.addr)
                    return right({ nodeAddress: node.addr, ...res })
                } catch (err) {
                    return left({ nodeAddr: node.addr, error: JSON.stringify(err) })
                }
            })
        )
        return topics
    }

    const fetchTopicsAndSet = () => fetchTopics()
        .then((res) => {
            let flatTopics: Either<{ nodeAddr: string, error: string }, FlatTopic>[] = []
            for (const response of res) {
                if (isLeft(response)) {
                    flatTopics = flatTopics.concat(response)
                } else {
                    flatTopics = flatTopics.concat(fromTopicLs(response.right).map(right))
                }

            }
            setTopics(flatTopics)
            return res.length
        })

    const toggle = (ix: number) => {
        setCheckedIxs((checkedIxs) => {
            if (checkedIxs.has(ix)) {
                checkedIxs.delete(ix)
            } else {
                checkedIxs.add(ix)
            }
            return new Set(checkedIxs)
        })
    }

    const deleteTopics = async () => {
        let deleted = []
        for (const index of checkedIxs) {
            let topic = topics[index]
            // This should always be true because the user can't select error topics
            if (isRight(topic)) {
                let deleteResult = await deleteTopic(topic.right.nodeAddress, topic.right.name)
                deleted.push({ name: topic.right.name, ...deleteResult })
            }
        }
        await fetchTopicsAndSet()
        setDeletedTopics(deleted)
    }

    useEffect(() => {
        fetchTopicsAndSet()
    }, [])

    useEffect(() => {
        setCheckedIxs(new Set())
    }, [topics])

    return (
        <Layout title="Topic Management">
            <div className="bg-white rounded p-4 min-h-full w-full min-w-full max-w-full overflow-hidden flex flex-col items-stretch h-full">
                <LsResults topics={topics} checkedIxs={checkedIxs} toggle={toggle}></LsResults>
                <Button
                    color="red"
                    disabled={checkedIxs.size <= 0}
                    onClick={deleteTopics}
                >Delete selected topics</Button>
                <div className="flex-grow mt-6 border-b border-l border-r rounded-md mb-1 text-xs flex flex-col">
                    <DeleteHeaderRow></DeleteHeaderRow>
                    <div className="flex-grow flex-shrink h-1 overflow-y-scroll overflow-x-hidden">
                        {deletedTopics.map((topic) => (
                            <DeleteResultRow
                                deleted={topic}
                            ></ DeleteResultRow>
                        ))}
                    </div>
                </div>
            </div>
        </Layout >
    )
}

export default Screen
