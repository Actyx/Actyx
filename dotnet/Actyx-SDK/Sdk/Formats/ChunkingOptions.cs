using System;

namespace Actyx.Sdk.Formats
{
    public struct ChunkingOptions
    {
        private int? maxChunkSize;
        /** Maximum chunk size. Defaults to 1000, if null */
        public int? MaxChunkSize
        {
            get => maxChunkSize;
            set => maxChunkSize = value < 0 ? 0 : value;
        }

        /**
         * Maximum duration (in ms) a chunk of events is allowed to grow, before being passed to the callback.
         * Defaults to 5, if null
         */
        public TimeSpan? MaxChunkTime { get; set; }
    }

}
