# senc

[senc](https://docs.senc.sh) (seh-nn-see) is a [hermetic](https://bazel.build/basics/hermeticity)
[TypeScript](https://www.typescriptlang.org/) interpreter for generating Infrastructure as Code (IaC). Use a familiar,
type-safe programming language to define and provision infrastructure, with protections that make your code easy to
debug and test.


## Technology

`senc` is built in [Rust](https://www.rust-lang.org/), and embeds [the Deno runtime](https://deno.com) for the
TypeScript runtime using the [deno_core crate](https://docs.rs/crate/deno_core/latest).


## What is Hermeticity?

[Hermeticity](https://bazel.build/basics/hermeticity) is the concept of a fully isolated build system that ensures the
output of a computation is always the same for the same input, regardless of the runtime environment. This is a concept
popularized in tools like [Bazel](https://bazel.build/) and [Jsonnet](https://jsonnet.org/), where hermeticity allowed
these systems to be super fast by enabling parallelism and aggressive caching in the process.

Hermeticity also has benefits in reproducibility, where it makes it really easy to analyze failing builds since there is
no dynamicism in the failure. Reproducing a failing build locally is as easy as pulling down the input sources and
retrying the build.

`senc` is an almost-hermetic runtime for TypeScript. It is "almost" because it exposes some limited access to the
environment, namely access to the file system (for code modularization) and stdout/stderr. However, it does not give
any other environmental access (e.g., network calls, environment variables, etc).


## Why `senc` over Pulumi or CDK?

Using TypeScript to provision and manage infrastructure is not a new concept. Existing tools such as
[Pulumi](https://www.pulumi.com/) and [CDK](https://aws.amazon.com/cdk/) already give you the ability to write
infrastructure code in TypeScript and provision it directly without external dependencies. So why bother with an extra
compilation step?

The main reason for this is because all these tools turn general purpose programming languages into an abstraction on
top of an underlying language for managing infrastructure. For Pulumi, this is a proprietary representation implemented
by the engine, which then gets reflected into the actual infrastructure. For CDK, this is either CloudFormation or
Terraform.

The challenge with the existing tools is that they hide away the intricacies of the underlying representation, making it
really difficult to trace down bugs in your code. When something goes wrong, it is oftentimes a nightmare to determine
if an issue is caused by a bug in the cloud layer, a bug in the infrastructure representation layer, or a bug in the top
TypeScript layer.

Another issue is that both Pulumi and CDK do not limit users in the TypeScript layer. For the most part, you can do
anything in the TypeScript layer, including reaching out to AWS APIs to inspect existing infrastructure. The cost of
this freedom is that it makes it difficult to test and develop against this code, since now you need to stand up actual
infrastructure. Depending on your runtime, this can also add overhead to credentials management. For example, if you
were using Terraform Cloud (TFC), you would need to first compile your infrastructure using `cdktf synth`, and then have
TFC deploy the compiled down code. If you have network dependent code in the TypeScript layer, then you would need to
share your credentials with both the CI system running `cdktf synth`, and TFC, expanding the surface area.

You can always restrict your team from using these features and have the same effect. However, in practice, if there is
a way to do something, it will always be used.

`senc` addresses both of these concerns by using an explicit hermetic compilation process. `senc` does not directly
provision infrastructure, delegating that task to the underlying infrastructure representation (either Terraform/OpenTofu,
or Kubernetes). This has a few advantages:

- Because the infrastructure provisioning step is explicit, it's very easy to trace down if a bug is from the Terraform
  code or TypeScript code. You can either introspect the generated code, or try running it directly yourself.
- `senc` is a hermetic runtime, and thus there is no way to write code that depends on the environment. This means that:
    - You can easily troubleshoot failing builds by rerunning locally with the same source.
    - You can run the compilation step without any credentials. Only share the credentials with your provisioning
      pipeline.
    - Testing can be done solely through introspection of the generated code. A typical testing pipeline would:
        1. Run `senc` to generate the IaC.
        1. Run validation to ensure the generated code is sound (e.g., `terraform validate`).
        1. Run a contract checker like [OPA](https://www.openpolicyagent.org/) or [CUE](https://cuelang.org/) to ensure
           the specific settings are set.

- Since `senc` doesn't handle the provisioning aspect, you can natively integrate with any of the Terraform runtimes,
  such as Terraform Cloud, Spacelift, env0, or Terraform/OpenTofu workflows on GitHub Actions.


## Why `senc` over Terraform / OpenTofu?

`senc` allows you to use TypeScript to provision and manage infrastructure. Although it does not give you the full range
of power behind the general purpose programming language (due to the hermeticity), it does give you access to the
expressiveness of the underlying programming language. This should be much more familiar to anyone who has experience
with general purpose programming languages than a DSL like HCL.

`senc` does not limit you from features available to Terraform/OpenTofu. Since `senc` is a code generator at heart, as
long as you generate the necessary Terrraform/OpenTofu code, you can use any feature or construct available.

However, by using a higher level language to generate the underlying Terraform/OpenTofu code, it allows you to
workaround certain limitations of HCL, most notably:
- You can interpolate constructs that can not be dynamically interpolated in HCL (e.g.,
  [lifecycle](https://developer.hashicorp.com/terraform/language/meta-arguments/lifecycle#literal-values-only) and
  [backend](https://developer.hashicorp.com/terraform/language/settings/backends/configuration)).
- You can reuse blocks that typically can't be reused (e.g.,
  [provider](https://developer.hashicorp.com/terraform/language/modules/develop/providers)).


## Why the name `senc`?

`senc` (pronounced seh-nn-see) comes from the word 仙人 (sen-nin) in Japanese, which itself is derived from
仙 (Xian) in Chinese. 仙人 refers to an immortal wizard or sage that is living as a hermit, typically in the mountains.
Note that the 人 character means "person" or "human."

The `c` in `senc` on the other hand means "compiler."

Putting all this together, `senc` can be translated to mean "compiler that is a hermit," which seems fitting for a
hermetic compiler.
