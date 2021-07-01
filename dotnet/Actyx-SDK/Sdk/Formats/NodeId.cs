namespace Actyx.Sdk.Formats
{
    public class NodeId
    {
        private readonly string nodeId;
        public NodeId(string nodeId)
        {
            this.nodeId = nodeId;
        }

        public override string ToString()
        {
            return nodeId;
        }

        public bool IsOwn(string stream) => stream.StartsWith(nodeId);
    }
}
