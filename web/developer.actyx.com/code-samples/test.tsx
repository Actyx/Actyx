const mdContent = `
~~~bash
# Request

curl \\
    -s -X "GET" \\
    -H "Authorization: Bearer $AUTH_TOKEN" \\
    -H "Accept: application/json" \\
    http://localhost:4454/api/v2/events/offsets | jq .

~~~
\`\`\`json
# Response

{
    "uAQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA.2": 57,
    "yjbwMjEteMT9Em8sGFwwde7kAGgJDxpTLJZZTxvduuKW.5": 60
}

\`\`\`

`

export default mdContent
