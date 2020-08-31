using System;
using FluentAssertions;
using Xunit;

namespace Sdk.Tests {
  public class AdderTets {
    [Fact]
    public void Adder_Should_Add_Numbers() {
      Adder.Add(1, 3).Should().Equals(4);
    }
  }
}
