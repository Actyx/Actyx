using System;
using System.Collections.Generic;
using System.Linq;

namespace Actyx
{
    // A selection of tags requirements, combined with AND and OR.
    // This is called "Where<E>" in TypeScript.
    public class From<E> : IFrom<E>
    {

        private readonly List<RawTagSet> underlyingSets;
        public List<RawTagSet> UnderlyingSets { get => underlyingSets; }

        public static From<E> AnyOf(params IFrom<E>[] AnyOfThese)
        {
            return new From<E>(AnyOfThese);
        }

        public From(params IFrom<E>[] AnyOfThese) : this(AnyOfThese.SelectMany(x => x.UnderlyingSets).ToList())
        {

        }


        public From(List<From<E>> AnyOfThese)
        {
            this.underlyingSets = AnyOfThese.SelectMany(x => x.underlyingSets).ToList();
        }

        protected From(List<RawTagSet> AnyOfThese)
        {
            this.underlyingSets = AnyOfThese;
        }

        public From<object> Or(IFrom<object> Other)
        {
            var copy = new List<RawTagSet>(underlyingSets);
            copy.AddRange(Other.UnderlyingSets);
            return new From<object>(copy);
        }

        public From<E> Or(IFrom<E> Other)
        {
            var copy = new List<RawTagSet>(underlyingSets);
            copy.AddRange(Other.UnderlyingSets);
            return new From<E>(copy);
        }


        public string ToAql()
        {
            IEnumerable<string> parts = UnderlyingSets.Select(tagSet => tagSet.ToTagExpr());
            return "FROM " + string.Join(" | ", parts);
        }
    }
}
