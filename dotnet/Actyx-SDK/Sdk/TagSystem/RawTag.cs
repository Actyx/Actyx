using System;
using System.Collections.Generic;

namespace Actyx
{
    // A raw tag with type parameter swallowed.
    // This is needed because otherwise ITags would need 2 type parameters and that would be too
    // confusing.
    // So what we do is omit a compile-time check that we KNOW would always succeed, at the cost of
    // a runtime check that we also KNOW will always succeed.
    public sealed class RawTag
    {
        internal static List<string> NamedSubSpace(string rawTag, string sub)
        {
            return new List<string> { rawTag, rawTag + ':' + sub };
        }

        internal static RawTag Create(string rawTagString) => new RawTag(rawTagString);

        internal static RawTag Create<E>(
            string rawTagString, Func<E, string> extractId
        )
        {
            string genericExtractId(object eventData)
            {

                return eventData switch
                {
                    null => null,
                    E e => extractId(e),
                    _ =>
                        // This should really not happen, is probably fine if we throw an exception.
                        throw new Exception($"Encountered unexpected concrete type with data {eventData} when trying to automatically extract id for tag {rawTagString}, please file a bug report."),
                };
            }

            return new RawTag(rawTagString, genericExtractId);
        }


        private readonly string rawTagString;
        private readonly Func<object, string> extractId;

        private RawTag(string rawTagString) : this(rawTagString, (_e) => null) { }

        private RawTag(string rawTagString, Func<object, string> extractId)
        {
            this.rawTagString = rawTagString;
            this.extractId = extractId;
        }

        internal List<string> AutoExtract(object eventData)
        {
            string id = this.extractId(eventData);
            // User supplied extractId impl is allowed to return null.
            if (id != null)
            {
                return NamedSubSpace(rawTagString, id);
            }
            return new List<string> { rawTagString };
        }

        internal string Escape()
        {
            // "'" + tag.tag.replace(/'/g, "''") + "'"
            return '\'' + rawTagString.Replace("'", "''") + '\'';
        }
    }
}
