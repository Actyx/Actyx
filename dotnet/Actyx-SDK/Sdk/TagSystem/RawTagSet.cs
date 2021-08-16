using System;
using System.Collections.Generic;
using System.Linq;

namespace Actyx
{
    // We swallow the type parameter to make our life easier internally.
    public sealed class RawTagSet
    {
        internal static RawTagSet Singleton(string rawTagString, bool onlyLocal)
        {
            return new RawTagSet(onlyLocal, new List<RawTag> { RawTag.Create(rawTagString) });
        }

        internal static RawTagSet Singleton(RawTag rawTag, bool onlyLocal)
        {
            return new RawTagSet(onlyLocal, new List<RawTag> { rawTag });
        }

        internal static RawTagSet Create(IEnumerable<RawTag> rawTags, bool onlyLocal)
        {
            return new RawTagSet(onlyLocal, new List<RawTag>(rawTags));
        }

        internal readonly List<RawTag> rawTags;
        internal readonly bool onlyLocal;

        private RawTagSet(bool onlyLocal, List<RawTag> rawTags)
        {
            this.onlyLocal = onlyLocal;
            this.rawTags = rawTags;
        }

        internal RawTagSet CopyWith(RawTag anotherTag)
        {
            var copy = new List<RawTag>(rawTags);
            copy.Add(anotherTag);
            return new RawTagSet(this.onlyLocal, copy);
        }

        internal RawTagSet CopyWith(RawTagSet moreTags)
        {
            var copy = new List<RawTag>(rawTags);
            copy.AddRange(moreTags.rawTags);
            return new RawTagSet(this.onlyLocal || moreTags.onlyLocal, copy);
        }

        internal RawTagSet Local()
        {
            if (this.onlyLocal)
            {
                return this;
            }
            return new RawTagSet(true, this.rawTags);
        }

        internal List<string> AutoExtract(object eventData)
        {
            return this.rawTags.SelectMany(x => x.AutoExtract(eventData)).ToList();
        }

        internal string ToTagExpr()
        {
            var escaped = this.rawTags.Select(x => x.Escape());
            var joined = String.Join(" & ", escaped);
            return onlyLocal ? joined + " & isLocal" : joined;
        }
    }
}
