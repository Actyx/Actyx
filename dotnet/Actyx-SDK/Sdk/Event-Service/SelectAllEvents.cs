namespace Actyx
{
    public sealed class SelectAllEvents : IEventSelection
    {
        private static readonly SelectAllEvents instance = new();

        // Explicit static constructor to tell C# compiler
        // not to mark type as beforefieldinit
        static SelectAllEvents()
        {
        }

        private SelectAllEvents()
        {
        }

        public static SelectAllEvents Instance => instance;

        public string ToAql() => "allEvents";
    }
}
