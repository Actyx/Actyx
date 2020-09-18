using Actyx;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;

namespace Sdk.IntegrationTests
{
    class Tags<E>
    {
        private readonly List<string> rawTags = new List<string>();

        public Tags(IEnumerable<string> tags)
        {
            this.rawTags.AddRange(tags);
        }

        public Tags<ESub> and<ESub>(Tags<ESub> other) where ESub : E
        {
            return new Tags<ESub>(rawTags.Concat(other.rawTags));
        }
    }

    class Tag<E> : Tags<E>
    {
        private readonly string rawTag;

        public Tag(string rawTag) : base(new string[] { rawTag })
        {
            this.rawTag = rawTag;
        }

        Tags<E> withId(string id)
        {
            return new Tags<E>(new string[] { this.rawTag, this.rawTag + ':' + id });
        }
    }



    interface EventHandler<S, in E>
    {
        S onEvent(S oldState, E eventPayload);
    }

    interface IUpdatedBy<in E>
    {
        void updateWith(E eventPayload);
    }

    interface IUpdateState<S>
    {
        S updateState(S oldState);
    }

    interface FishBuilder<S>
    {
        FishBuilder<S> subscribeTo<E>(Tags<E> subscription, EventHandler<S, E> handler);

        FishBuilder<S> subscribeTo<E>(Tags<E> subscription) where E : IUpdateState<S>;
    }

    interface FishBuilderX<S, X> : FishBuilder<S> where S : IUpdatedBy<X>
    {
        FishBuilderX<S, X> subscribeToX(Tags<X> subscription);
    }


    interface Ponder
    {
        FishBuilder<S> fish<S>(string fishId, S initialState);

        FishBuilderX<S, X> fish<S, X>(string fishId, S initialState) where S : IUpdatedBy<X>;
    }


    class Fooo
    {
        static async Task Ok(Ponder p)
        {


        }
    }

    // class FishBuilder<S> : FishBuilder<S>
    // {
    //     private readonly string fishId;
    //     private readonly S initialState;

    //     public FishBuilder(string fishId, S initialState)
    //     {
    //         this.fishId = fishId;
    //         this.initialState = initialState;
    //     }

    //     FishBuilder<S> subscribeTo(Tags<E> subscription, EventHandler<S, E> handler)
    //     {
    //         return this;
    //     }

    //     FishBuilder<S> subscribeTo(Tags<E> subscription) where S : IUpdatedBy<E>
    //     {
    //         return this;
    //     }

    //     FishBuilder<S> subscribeTo(Tags<E> subscription) where E : IUpdateState<S>
    //     {
    //         return this;
    //     }
    // }

}
