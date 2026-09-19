module.exports = {
  dependency: {
    platforms: {
      macos: null,
      windows: {
        sourceDir: 'windows',
        solutionFile: 'ServoKit.sln',
        projects: [
          {
            projectFile: 'ServoKit\\ServoKit.vcxproj',
            directDependency: true,
          },
        ],
      },
    },
  },
};
