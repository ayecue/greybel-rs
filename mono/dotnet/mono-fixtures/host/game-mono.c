#include <dlfcn.h>
#include <stdio.h>

typedef void (*set_dirs_fn)(const char *assembly_dir, const char *config_dir);
typedef void (*set_assemblies_path_fn)(const char *path);
typedef void (*config_parse_fn)(const char *filename);
typedef void *(*jit_init_version_fn)(const char *domain, const char *version);
typedef void *(*domain_assembly_open_fn)(void *domain, const char *name);
typedef int (*jit_exec_fn)(void *domain, void *assembly, int argc, char **argv);

static void *symbol(void *library, const char *name) {
    void *found = dlsym(library, name);
    if (!found) {
        fprintf(stderr, "game-mono: %s is missing: %s\n", name, dlerror());
    }
    return found;
}

int main(int argc, char **argv) {
    if (argc < 5) {
        fprintf(stderr, "usage: game-mono <libmono> <managed dir> <etc dir> <program.exe> [args...]\n");
        return 2;
    }
    void *library = dlopen(argv[1], RTLD_NOW | RTLD_GLOBAL);
    if (!library) {
        fprintf(stderr, "game-mono: %s\n", dlerror());
        return 1;
    }
    set_dirs_fn set_dirs = (set_dirs_fn)symbol(library, "mono_set_dirs");
    set_assemblies_path_fn set_assemblies_path = (set_assemblies_path_fn)symbol(library, "mono_set_assemblies_path");
    config_parse_fn config_parse = (config_parse_fn)symbol(library, "mono_config_parse");
    jit_init_version_fn jit_init_version = (jit_init_version_fn)symbol(library, "mono_jit_init_version");
    domain_assembly_open_fn domain_assembly_open = (domain_assembly_open_fn)symbol(library, "mono_domain_assembly_open");
    jit_exec_fn jit_exec = (jit_exec_fn)symbol(library, "mono_jit_exec");
    if (!set_dirs || !set_assemblies_path || !config_parse || !jit_init_version || !domain_assembly_open || !jit_exec) {
        return 1;
    }

    set_dirs(argv[2], argv[3]);
    set_assemblies_path(argv[2]);
    config_parse(NULL);
    void *domain = jit_init_version("mono-fixtures", "v4.0.30319");
    void *assembly = domain_assembly_open(domain, argv[4]);
    if (!assembly) {
        fprintf(stderr, "game-mono: cannot open %s\n", argv[4]);
        return 1;
    }
    return jit_exec(domain, assembly, argc - 4, argv + 4);
}
