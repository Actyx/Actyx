import { Layout } from "../components/Layout"
import React, { useEffect, useState } from "react"
import { Cell, ColWidth, Row, TruncatableString } from "./Query"
import { useAppState } from "../app-state"
import { getTopicList } from "client/util"
import { TopicLsResponse } from "common/types"


const HeaderRow = () => {
    const cells: [string, ColWidth][] = [
        ["Node", "112"],
        ["Topic", "32"],
        ["Size", "32"],
        ["Active", "16"]
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
                    isLast={text === "Active"} >
                    <TruncatableString>{text}</TruncatableString>
                </Cell>
            ))
            }
        </Row >
    )
}

const ResultRow = ({ topic }: { topic: TopicLsResponse }) => {
    const rows: [string, string, ColWidth][][] = Object.keys(topic.topics).map((topicName) => {
        return [
            ["node", topic.nodeId, "112"],
            ["topic", topicName, "32"],
            ["size", topic.topics[topicName].toString(), "32"],
            ["active", topic.activeTopic === topicName ? "✓" : "", "16"]
        ]
    })

    return (
        <div>{
            rows.map((row, ix) => (
                <React.Fragment key={`row${ix}`}>
                    <Row height="7" accentColor="blue" expandableObject={false} isChecked={false}>
                        {(onClick, rowIsExpanded) =>
                            row.map(([keyPrefix, text, width]) => (
                                <Cell
                                    key={`${keyPrefix}+${text}`}
                                    height="7"
                                    width={width}
                                    rowIsExpanded={rowIsExpanded}
                                    onClick={onClick}
                                    isLast={text === "active"}>
                                    <TruncatableString>{text}</TruncatableString>
                                </Cell>
                            ))
                        }
                    </Row>
                </React.Fragment>
            ))
        }</div>
    )
}



const Screen: React.FC<{}> = () => {
    const {
        data: { nodes },
        actions: { getTopicList, deleteTopic }
    } = useAppState()

    const getTopics = async () => {
        let topics = await Promise.all(
            nodes.map((node) => {
                console.log(node.addr)
                return getTopicList(node.addr)
            })
        )
        return topics
    }

    const [topics, setTopics] = useState<null | TopicLsResponse[]>(null);

    useEffect(() => {
        getTopics()
            .then((res) => setTopics(res))
    }, [])



    return (
        <Layout title="Topic Management">
            <div className="bg-white rounded p-4 min-h-full w-full min-w-full max-w-full overflow-hidden flex flex-col items-stretch h-full">
                <HeaderRow></HeaderRow>
                {topics?.map((topic) => (
                    <ResultRow topic={topic}></ ResultRow >
                ))}
            </div>
        </Layout>
    )
}

export default Screen
