using System;

namespace Actyx
{
    // A specific Tag, that optionally can automatically extract Ids from events of its type.
    public class Tag<E> : Tags<E>
    {

        private readonly string tagString;

        public Tag(string tagString) : base(RawTagSet.Singleton(tagString, false))
        {
            this.tagString = tagString;
        }

        public Tag(string tagString, Func<E, string> autoExtractId) : base(RawTagSet.Singleton(RawTag.Create<E>(tagString, autoExtractId), false))
        {
            this.tagString = tagString;
        }

        public Tags<E> WithId(string id)
        {
            return new Tags<E>(RawTag.NamedSubSpace(tagString, id), false);
        }
    }
}
