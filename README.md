# senc

[SenC](https://docs.senc.sh) (seh-nn-see) is a [hermetic](https://bazel.build/basics/hermeticity)
[TypeScript](https://www.typescriptlang.org/) interpreter for generating Infrastructure as Code (IaC). Use a familiar,
type-safe programming language to define and provision infrastructure, with protections that make your code easy to
debug and test.


## What is Hermeticity?

[Hermeticity](https://bazel.build/basics/hermeticity) is the concept of a fully isolated build system that ensures the
output of a computation is always the same for the same input, regardless of the runtime environment. This was a concept
that was first popularized with [Bazel](https://bazel.build/), where hermeticity allowed the build system to be super
fast by enabling parallelism and aggressive caching in the build process.

Hermeticity also has benefits in reproducibility, where it makes it really easy to analyze failing builds since there is
no dynamicism in the failure. Reproducing a failing build locally is as easy as pulling down the input sources and
retrying the build.


## Why Senc over Pulumi or CDK?

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

You can always restrict your team from using these features and have the same effect. However, in practice, bad
practices always sneak their way in if there is a way to do it.

Senc addresses both of these concerns by using an explicit hermetic compilation process. Senc does not directly
provision infrastructure, delegating that task to the underlying infrastructure representation (either Terraform/OpenTF,
or Kubernetes). This has a few advantages:

- Because the infrastructure provisioning step is explicit, it's very easy to trace down if a bug is from the Terraform
  code or TypeScript code. You can either introspect the generated code, or try running it directly yourself.
- `senc` is a hermetic runtime, and thus there is no way to write code that depends on the environment. This means that
  you can easily troubleshoot failing builds by rerunning locally with the same source.
- The hermetic runtime also ensures you can run the compilation step without any credentials. Only share the credentials
  with your provisioning pipeline.
- Since `senc` doesn't handle the provisioning aspect, you can natively integrate with any of the Terraform runtimes,
  such as Terraform Cloud, Spacelift, env0, or Terraform/OpenTF workflows on GitHub Actions.


## Why the name `senc`?

SenC (pronounced seh-nn-see) comes from the word 仙 (sen) in Japanese, which itself is derived from the same character
in Chinese, Xian. 仙 refers to an immortal wizard or sage that is living as a hermit, typically in the mountains. The
`C` in `SenC` refers to compilation. Combining it together, this translates to a `hermit compiler`, which seems fitting
for a hermetic compiler tool.
